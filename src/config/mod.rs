use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Configuration {
    pub interval: Option<IntervalConfig>,
    pub sonarr: Option<SonarrConfig>,
    pub radarr: Option<RadarrConfig>,
    pub plex: Option<PlexConfig>,
    pub delete: Option<DeleteConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IntervalConfig {
    pub seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SonarrConfig {
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(rename = "apikey")]
    pub api_key: String,
    #[serde(rename = "qualityProfile")]
    pub quality_profile: Option<String>,
    #[serde(rename = "rootFolder")]
    pub root_folder: Option<String>,
    #[serde(rename = "bypassIgnored")]
    pub bypass_ignored: Option<bool>,
    #[serde(rename = "seasonMonitoring")]
    pub season_monitoring: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RadarrConfig {
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(rename = "apikey")]
    pub api_key: String,
    #[serde(rename = "qualityProfile")]
    pub quality_profile: Option<String>,
    #[serde(rename = "rootFolder")]
    pub root_folder: Option<String>,
    #[serde(rename = "bypassIgnored")]
    pub bypass_ignored: Option<bool>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlexConfig {
    pub token: String,
    #[serde(rename = "skipfriendsync")]
    pub skip_friend_sync: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteConfig {
    pub movie: Option<bool>,
    #[serde(rename = "endedShow")]
    pub ended_show: Option<bool>,
    #[serde(rename = "continuingShow")]
    pub continuing_show: Option<bool>,
    pub interval: Option<DeleteIntervalConfig>,
    #[serde(rename = "deleteFiles")]
    pub delete_files: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteIntervalConfig {
    pub days: u64,
}

impl Configuration {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Configuration = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn refresh_interval(&self) -> Duration {
        Duration::from_secs(
            self.interval
                .as_ref()
                .map(|i| i.seconds)
                .unwrap_or(10)
        )
    }

    pub fn delete_interval(&self) -> Duration {
        Duration::from_secs(
            self.delete
                .as_ref()
                .and_then(|d| d.interval.as_ref())
                .map(|i| i.days * 24 * 60 * 60)
                .unwrap_or(7 * 24 * 60 * 60)
        )
    }
}