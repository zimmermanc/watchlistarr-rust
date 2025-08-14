mod config;
mod http;
mod models;
mod plex;
mod radarr;
mod sonarr;

use anyhow::Result;
use clap::Parser;
use config::Configuration;
use http::HttpClient;
use models::ItemType;
use plex::PlexClient;
use radarr::RadarrClient;
use sonarr::SonarrClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    info!("Starting Watchlistarr Rust v0.1.0");

    // Load configuration
    let config = Arc::new(Configuration::from_file(&cli.config)?);
    info!("Configuration loaded from: {}", cli.config);

    // Initialize HTTP client
    let http_client = HttpClient::new();

    // Start sync tasks
    let sync_tasks = vec![
        tokio::spawn(ping_token_sync(Arc::clone(&config), http_client.clone())),
        tokio::spawn(plex_rss_sync(Arc::clone(&config), http_client.clone())),
        tokio::spawn(plex_full_sync(Arc::clone(&config), http_client.clone())),
        tokio::spawn(plex_delete_sync(Arc::clone(&config), http_client.clone())),
    ];

    // Wait for all tasks (they run forever)
    for task in sync_tasks {
        if let Err(e) = task.await? {
            error!("Sync task failed: {}", e);
        }
    }

    Ok(())
}

async fn ping_token_sync(config: Arc<Configuration>, http_client: HttpClient) -> Result<()> {
    let mut interval = interval(Duration::from_secs(24 * 60 * 60)); // 24 hours
    
    loop {
        interval.tick().await;
        
        if let Some(ref plex_config) = config.plex {
            info!("Running token ping sync");
            
            let plex_client = PlexClient::new(http_client.clone(), plex_config.clone());
            
            match plex_client.get_watchlist().await {
                Ok(_) => debug!("Token ping successful"),
                Err(e) => warn!("Token ping failed: {}", e),
            }
        }
    }
}

async fn plex_rss_sync(config: Arc<Configuration>, http_client: HttpClient) -> Result<()> {
    let refresh_interval = config.refresh_interval();
    let mut interval = interval(refresh_interval);
    
    loop {
        interval.tick().await;
        
        if let Err(e) = run_sync(&config, &http_client, false).await {
            error!("RSS sync failed: {}", e);
        }
    }
}

async fn plex_full_sync(config: Arc<Configuration>, http_client: HttpClient) -> Result<()> {
    let mut interval = interval(Duration::from_secs(19 * 60)); // 19 minutes
    
    loop {
        interval.tick().await;
        
        if let Err(e) = run_sync(&config, &http_client, true).await {
            error!("Full sync failed: {}", e);
        }
    }
}

async fn plex_delete_sync(config: Arc<Configuration>, http_client: HttpClient) -> Result<()> {
    let delete_interval = config.delete_interval();
    let mut interval = interval(delete_interval);
    
    loop {
        interval.tick().await;
        
        if let Some(ref delete_config) = config.delete {
            if delete_config.movie.unwrap_or(false) 
                || delete_config.ended_show.unwrap_or(false) 
                || delete_config.continuing_show.unwrap_or(false) 
            {
                info!("Running delete sync");
                if let Err(e) = run_delete_sync(&config, &http_client).await {
                    error!("Delete sync failed: {}", e);
                }
            }
        }
    }
}

async fn run_sync(config: &Configuration, http_client: &HttpClient, full_sync: bool) -> Result<()> {
    let Some(ref plex_config) = config.plex else {
        warn!("No Plex configuration found, skipping sync");
        return Ok(());
    };

    info!("Running {} sync", if full_sync { "full" } else { "RSS" });
    
    let plex_client = PlexClient::new(http_client.clone(), plex_config.clone());
    
    // Get watchlist items
    let mut watchlist_items = plex_client.get_watchlist().await?;
    
    if !plex_config.skip_friend_sync.unwrap_or(false) && full_sync {
        let friends_items = plex_client.get_friends_watchlists().await?;
        watchlist_items.extend(friends_items);
    }

    info!("Found {} items in watchlist", watchlist_items.len());

    // Process items
    for watchlist_item in watchlist_items {
        let item = &watchlist_item.item;
        
        match item.item_type {
            ItemType::Movie => {
                if let Some(ref radarr_config) = config.radarr {
                    let radarr_client = RadarrClient::new(http_client.clone(), radarr_config.clone());
                    if let Err(e) = radarr_client.add_movie(item).await {
                        error!("Failed to add movie to Radarr: {}", e);
                    }
                }
            }
            ItemType::Show => {
                if let Some(ref sonarr_config) = config.sonarr {
                    let sonarr_client = SonarrClient::new(http_client.clone(), sonarr_config.clone());
                    if let Err(e) = sonarr_client.add_series(item).await {
                        error!("Failed to add series to Sonarr: {}", e);
                    }
                }
            }
        }
        
        // Small delay between requests to be respectful
        sleep(Duration::from_millis(100)).await;
    }

    info!("Sync completed");
    Ok(())
}

async fn run_delete_sync(_config: &Configuration, _http_client: &HttpClient) -> Result<()> {
    info!("Delete sync functionality not yet implemented");
    Ok(())
}
