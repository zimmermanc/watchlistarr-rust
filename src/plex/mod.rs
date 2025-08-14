use crate::config::PlexConfig;
use crate::http::HttpClient;
use crate::models::{Item, ItemType, WatchlistItem};
use anyhow::Result;
use tracing::{debug, error, info, instrument, warn};

pub struct PlexClient {
    http: HttpClient,
    config: PlexConfig,
}

impl PlexClient {
    pub fn new(http: HttpClient, config: PlexConfig) -> Self {
        Self { http, config }
    }

    #[instrument(skip(self))]
    pub async fn get_watchlist(&self) -> Result<Vec<WatchlistItem>> {
        info!("Fetching Plex watchlist");
        
        let url = format!(
            "https://metadata.provider.plex.tv/library/sections/watchlist/all?X-Plex-Token={}",
            self.config.token
        );

        match self.http.get(&url).await {
            Ok(response) => {
                let xml_text = response.text().await?;
                debug!("Received XML response: {} chars", xml_text.len());
                
                let items = self.parse_xml_watchlist(&xml_text)?;
                
                info!("Retrieved {} watchlist items", items.len());
                Ok(items)
            }
            Err(e) => {
                error!("Failed to fetch Plex watchlist: {}", e);
                Err(e)
            }
        }
    }

    fn parse_xml_watchlist(&self, xml: &str) -> Result<Vec<WatchlistItem>> {
        let mut items = Vec::new();
        
        info!("Starting XML parsing for {} character XML", xml.len());
        
        // Find all Video elements (movies) - they contain type="movie"
        let mut start_pos = 0;
        while let Some(video_start) = xml[start_pos..].find("<Video ") {
            let actual_start = start_pos + video_start;
            if let Some(end_pos) = xml[actual_start..].find(">") {
                let element = &xml[actual_start..actual_start + end_pos + 1];
                
                // Check if this is a movie with the required attributes
                if element.contains("type=\"movie\"") && element.contains("title=") && element.contains("ratingKey=") {
                    if let (Some(title), Some(rating_key)) = (self.extract_title(element), self.extract_rating_key(element)) {
                        let year = self.extract_year(element);
                        let guid = self.extract_guid(element);
                        
                        let item = Item {
                            id: rating_key,
                            title: title.clone(),
                            year,
                            item_type: ItemType::Movie,
                            guid,
                            imdb_id: None,
                            tmdb_id: None,
                            tvdb_id: None,
                        };
                        
                        let watchlist_item = WatchlistItem {
                            item,
                            added_at: chrono::Utc::now(),
                            user_id: "self".to_string(),
                        };
                        
                        info!("Found movie: {} ({}) [Rating Key: {}]", 
                              title, 
                              year.map_or("Unknown".to_string(), |y| y.to_string()),
                              &watchlist_item.item.id);
                        items.push(watchlist_item);
                    }
                }
                start_pos = actual_start + end_pos + 1;
            } else {
                break;
            }
        }
        
        // Find all Directory elements (shows) - they contain type="show"
        let mut start_pos = 0;
        while let Some(dir_start) = xml[start_pos..].find("<Directory ") {
            let actual_start = start_pos + dir_start;
            if let Some(end_pos) = xml[actual_start..].find(">") {
                let element = &xml[actual_start..actual_start + end_pos + 1];
                
                // Check if this is a show with the required attributes
                if element.contains("type=\"show\"") && element.contains("title=") && element.contains("ratingKey=") {
                    if let (Some(title), Some(rating_key)) = (self.extract_title(element), self.extract_rating_key(element)) {
                        let year = self.extract_year(element);
                        let guid = self.extract_guid(element);
                        
                        let item = Item {
                            id: rating_key,
                            title: title.clone(),
                            year,
                            item_type: ItemType::Show,
                            guid,
                            imdb_id: None,
                            tmdb_id: None,
                            tvdb_id: None,
                        };
                        
                        let watchlist_item = WatchlistItem {
                            item,
                            added_at: chrono::Utc::now(),
                            user_id: "self".to_string(),
                        };
                        
                        info!("Found show: {} ({}) [Rating Key: {}]", 
                              title, 
                              year.map_or("Unknown".to_string(), |y| y.to_string()),
                              &watchlist_item.item.id);
                        items.push(watchlist_item);
                    }
                }
                start_pos = actual_start + end_pos + 1;
            } else {
                break;
            }
        }
        
        info!("XML parsing completed: found {} total items", items.len());
        Ok(items)
    }
    
    fn extract_title(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find("title=\"") {
            let start = start + 7; // Skip 'title="'
            if let Some(end) = line[start..].find('"') {
                return Some(line[start..start + end].to_string());
            }
        }
        None
    }
    
    fn extract_rating_key(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find("ratingKey=\"") {
            let start = start + 11; // Skip 'ratingKey="'
            if let Some(end) = line[start..].find('"') {
                return Some(line[start..start + end].to_string());
            }
        }
        None
    }
    
    fn extract_year(&self, line: &str) -> Option<i32> {
        if let Some(start) = line.find("year=\"") {
            let start = start + 6; // Skip 'year="'
            if let Some(end) = line[start..].find('"') {
                return line[start..start + end].parse().ok();
            }
        }
        None
    }
    
    fn extract_guid(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find("guid=\"") {
            let start = start + 6; // Skip 'guid="'
            if let Some(end) = line[start..].find('"') {
                return Some(line[start..start + end].to_string());
            }
        }
        None
    }

    #[instrument(skip(self))]
    pub async fn get_friends_watchlists(&self) -> Result<Vec<WatchlistItem>> {
        if self.config.skip_friend_sync.unwrap_or(false) {
            debug!("Skipping friends sync as configured");
            return Ok(Vec::new());
        }

        info!("Fetching friends' watchlists");
        warn!("Friends watchlist sync not yet implemented");
        Ok(Vec::new())
    }
}
