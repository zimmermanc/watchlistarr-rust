use crate::config::RadarrConfig;
use crate::http::HttpClient;
use crate::models::{Item, ItemType, QualityProfile, RootFolder, Tag};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, warn};

pub struct RadarrClient {
    http: HttpClient,
    config: RadarrConfig,
}

#[derive(Debug, Serialize)]
struct RadarrMovie {
    title: String,
    #[serde(rename = "originalTitle")]
    original_title: String,
    #[serde(rename = "sortTitle")]
    sort_title: String,
    year: i32,
    #[serde(rename = "tmdbId")]
    tmdb_id: Option<i32>,
    #[serde(rename = "imdbId")]
    imdb_id: Option<String>,
    #[serde(rename = "qualityProfileId")]
    quality_profile_id: i32,
    #[serde(rename = "rootFolderPath")]
    root_folder_path: String,
    #[serde(rename = "addOptions")]
    add_options: RadarrAddOptions,
    monitored: bool,
    tags: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RadarrAddOptions {
    #[serde(rename = "searchForMovie")]
    search_for_movie: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct RadarrLookupResult {
    title: String,
    #[serde(rename = "originalTitle")]
    original_title: String,
    #[serde(rename = "sortTitle")]
    sort_title: String,
    year: Option<i32>,
    #[serde(rename = "tmdbId", skip_serializing_if = "Option::is_none")]
    tmdb_id: Option<i32>,
    #[serde(rename = "imdbId", skip_serializing_if = "Option::is_none")]
    imdb_id: Option<String>,
    #[serde(flatten)]
    extra_fields: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct RadarrMovieSimple {
    #[serde(rename = "tmdbId")]
    tmdb_id: Option<i32>,
}

impl RadarrClient {
    pub fn new(http: HttpClient, config: RadarrConfig) -> Self {
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
    pub async fn get_movies(&self) -> Result<Vec<RadarrMovieSimple>> {
        let url = format!("{}/api/v3/movie?apikey={}", 
                         self.config.base_url, self.config.api_key);
        
        self.http.get_json(&url).await
    }

    #[instrument(skip(self, item))]
    pub async fn add_movie(&self, item: &Item) -> Result<()> {
        if item.item_type != ItemType::Movie {
            warn!("Attempted to add non-movie item to Radarr: {}", item.title);
            return Ok(());
        }

        info!("Adding movie to Radarr: {}", item.title);
        
        // First, lookup the movie to get TMDB ID and other metadata
        let lookup_result = self.lookup_movie(&item.title, item.year).await?;

        // Check if movie already exists in Radarr
        if let Some(tmdb_id) = lookup_result.tmdb_id {
            let existing_movies = self.get_movies().await?;
            if existing_movies.iter().any(|m| m.tmdb_id == Some(tmdb_id)) {
                info!("Movie '{}' (TMDB: {}) already exists in Radarr, skipping", lookup_result.title, tmdb_id);
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
                .unwrap_or_else(|| "/mnt/shared/movies".to_string())
        };

        let tag_ids = if let Some(ref tags) = self.config.tags {
            self.resolve_tag_ids(tags).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        info!("Using quality profile ID: {}, root folder: {}", quality_profile_id, root_folder_path);

        let movie = RadarrMovie {
            title: lookup_result.title.clone(),
            original_title: lookup_result.original_title,
            sort_title: lookup_result.sort_title,
            year: lookup_result.year.unwrap_or(0),
            tmdb_id: lookup_result.tmdb_id,
            imdb_id: lookup_result.imdb_id,
            quality_profile_id,
            root_folder_path,
            add_options: RadarrAddOptions {
                search_for_movie: true,
            },
            monitored: true,
            tags: tag_ids,
        };

        let url = format!("{}/api/v3/movie?apikey={}", 
                         self.config.base_url, self.config.api_key);
        
        match self.http.post_json::<serde_json::Value, _>(&url, &movie).await {
            Ok(_) => {
                info!("Successfully added movie: {}", lookup_result.title);
                Ok(())
            }
            Err(e) => {
                error!("Failed to add movie '{}': {}", lookup_result.title, e);
                Err(e)
            }
        }
    }

    #[instrument(skip(self))]
    async fn lookup_movie(&self, title: &str, year: Option<i32>) -> Result<RadarrLookupResult> {
        let search_term = if let Some(year) = year {
            format!("{} {}", title, year)
        } else {
            title.to_string()
        };
        
        let url = format!("{}/api/v3/movie/lookup?term={}&apikey={}", 
                         self.config.base_url, 
                         urlencoding::encode(&search_term),
                         self.config.api_key);
        
        info!("Looking up movie: {}", search_term);
        
        let results: Vec<RadarrLookupResult> = self.http.get_json(&url).await?;
        
        if let Some(result) = results.first() {
            info!("Found movie: {} (TMDB: {:?})", result.title, result.tmdb_id);
            Ok(result.clone())
        } else {
            Err(anyhow::anyhow!("Movie not found in lookup: {}", search_term))
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
