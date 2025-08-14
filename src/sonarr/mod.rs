use crate::config::SonarrConfig;
use crate::http::HttpClient;
use crate::models::{Item, ItemType, QualityProfile, RootFolder, Tag};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};

pub struct SonarrClient {
    http: HttpClient,
    config: SonarrConfig,
}

#[derive(Debug, Serialize)]
struct SonarrSeries {
    title: String,
    #[serde(rename = "sortTitle")]
    sort_title: String,
    year: i32,
    #[serde(rename = "tvdbId")]
    tvdb_id: Option<i32>,
    #[serde(rename = "imdbId")]
    imdb_id: Option<String>,
    #[serde(rename = "tmdbId")]
    tmdb_id: Option<i32>,
    #[serde(rename = "qualityProfileId")]
    quality_profile_id: i32,
    #[serde(rename = "rootFolderPath")]
    root_folder_path: String,
    #[serde(rename = "addOptions")]
    add_options: SonarrAddOptions,
    monitored: bool,
    tags: Vec<i32>,
}

#[derive(Debug, Serialize)]
struct SonarrAddOptions {
    monitor: String,
    #[serde(rename = "searchForMissingEpisodes")]
    search_for_missing_episodes: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct SonarrLookupResult {
    title: String,
    #[serde(rename = "sortTitle")]
    sort_title: String,
    year: Option<i32>,
    #[serde(rename = "tvdbId")]
    tvdb_id: Option<i32>,
    #[serde(rename = "imdbId")]
    imdb_id: Option<String>,
    #[serde(rename = "tmdbId")]
    tmdb_id: Option<i32>,
    #[serde(flatten)]
    extra_fields: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SonarrSeriesSimple {
    #[serde(rename = "tvdbId")]
    tvdb_id: Option<i32>,
    #[serde(rename = "tmdbId")]
    tmdb_id: Option<i32>,
}

impl SonarrClient {
    pub fn new(http: HttpClient, config: SonarrConfig) -> Self {
        Self { http, config }
    }

    #[instrument(skip(self))]
    pub async fn get_quality_profiles(&self) -> Result<Vec<QualityProfile>> {
        let url = format!("{}/api/v3/qualityprofile?apikey={}", 
                         self.config.base_url, self.config.api_key);
        
        self.http.get_json(&url).await
    }

    #[instrument(skip(self))]
    pub async fn get_root_folders(&self) -> Result<Vec<RootFolder>> {
        let url = format!("{}/api/v3/rootfolder?apikey={}", 
                         self.config.base_url, self.config.api_key);
        
        self.http.get_json(&url).await
    }

    #[instrument(skip(self))]
    pub async fn get_tags(&self) -> Result<Vec<Tag>> {
        let url = format!("{}/api/v3/tag?apikey={}", 
                         self.config.base_url, self.config.api_key);
        
        self.http.get_json(&url).await
    }

    #[instrument(skip(self))]
    pub async fn get_series(&self) -> Result<Vec<SonarrSeriesSimple>> {
        let url = format!("{}/api/v3/series?apikey={}", 
                         self.config.base_url, self.config.api_key);
        
        self.http.get_json(&url).await
    }

    #[instrument(skip(self))]
    async fn lookup_series(&self, title: &str, year: Option<i32>) -> Result<SonarrLookupResult> {
        let search_term = if let Some(year) = year {
            format!("{} {}", title, year)
        } else {
            title.to_string()
        };
        
        let url = format!("{}/api/v3/series/lookup?term={}&apikey={}", 
                         self.config.base_url, 
                         urlencoding::encode(&search_term),
                         self.config.api_key);
        
        info!("Looking up series: {}", search_term);
        
        let results: Vec<SonarrLookupResult> = self.http.get_json(&url).await?;
        
        if let Some(result) = results.first() {
            info!("Found series: {} (TVDB: {:?}, TMDB: {:?})", result.title, result.tvdb_id, result.tmdb_id);
            Ok(result.clone())
        } else {
            Err(anyhow::anyhow!("Series not found in lookup: {}", search_term))
        }
    }

    #[instrument(skip(self, item))]
    pub async fn add_series(&self, item: &Item) -> Result<()> {
        if item.item_type != ItemType::Show {
            warn!("Attempted to add non-show item to Sonarr: {}", item.title);
            return Ok(());
        }

        info!("Adding series to Sonarr: {}", item.title);

        // First, lookup the series to get TVDB/TMDB ID and other metadata
        let lookup_result = self.lookup_series(&item.title, item.year).await?;

        // Check if series already exists in Sonarr
        let existing_series = self.get_series().await?;
        
        // Check for duplicates using both TVDB and TMDB IDs
        if let Some(tvdb_id) = lookup_result.tvdb_id {
            if existing_series.iter().any(|s| s.tvdb_id == Some(tvdb_id)) {
                info!("Series '{}' (TVDB: {}) already exists in Sonarr, skipping", lookup_result.title, tvdb_id);
                return Ok(());
            }
        }
        
        if let Some(tmdb_id) = lookup_result.tmdb_id {
            if existing_series.iter().any(|s| s.tmdb_id == Some(tmdb_id)) {
                info!("Series '{}' (TMDB: {}) already exists in Sonarr, skipping", lookup_result.title, tmdb_id);
                return Ok(());
            }
        }

        let quality_profiles = self.get_quality_profiles().await?;
        let root_folders = self.get_root_folders().await?;
        
        let quality_profile_id = if let Some(ref profile_name) = self.config.quality_profile {
            quality_profiles
                .iter()
                .find(|p| p.name == *profile_name)
                .map(|p| p.id)
                .unwrap_or_else(|| {
                    warn!("Quality profile '{}' not found, using first available", profile_name);
                    quality_profiles.first().map(|p| p.id).unwrap_or(1)
                })
        } else {
            quality_profiles.first().map(|p| p.id).unwrap_or(1)
        };

        let root_folder_path = if let Some(ref folder) = self.config.root_folder {
            folder.clone()
        } else {
            root_folders
                .first()
                .map(|f| f.path.clone())
                .unwrap_or_else(|| "/tv".to_string())
        };

        let tag_ids = if let Some(ref tags) = self.config.tags {
            self.resolve_tag_ids(tags).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        info!("Using quality profile ID: {}, root folder: {}", quality_profile_id, root_folder_path);

        let series = SonarrSeries {
            title: lookup_result.title.clone(),
            sort_title: lookup_result.sort_title,
            year: lookup_result.year.unwrap_or(0),
            tvdb_id: lookup_result.tvdb_id,
            imdb_id: lookup_result.imdb_id,
            tmdb_id: lookup_result.tmdb_id,
            quality_profile_id,
            root_folder_path,
            add_options: SonarrAddOptions {
                monitor: self.config.season_monitoring.clone().unwrap_or_else(|| "all".to_string()),
                search_for_missing_episodes: true,
            },
            monitored: true,
            tags: tag_ids,
        };

        let url = format!("{}/api/v3/series?apikey={}", 
                         self.config.base_url, self.config.api_key);
        
        match self.http.post_json::<serde_json::Value, _>(&url, &series).await {
            Ok(_) => {
                info!("Successfully added series: {}", lookup_result.title);
                Ok(())
            }
            Err(e) => {
                error!("Failed to add series '{}': {}", lookup_result.title, e);
                Err(e)
            }
        }
    }

    async fn resolve_tag_ids(&self, tag_names: &[String]) -> Result<Vec<i32>> {
        let tags = self.get_tags().await?;
        Ok(tag_names
            .iter()
            .filter_map(|name| tags.iter().find(|t| t.label == *name).map(|t| t.id))
            .collect())
    }
}