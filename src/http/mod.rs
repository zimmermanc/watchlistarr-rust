use anyhow::Result;
use reqwest::{Client, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use std::time::Duration;
use tracing::{debug, error, instrument};

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("watchlistarr-rust/0.1.0")
            .build()
            .expect("Failed to create HTTP client");
        
        Self { client }
    }

    #[instrument(skip(self), fields(url = %url))]
    pub async fn get(&self, url: &str) -> Result<Response> {
        debug!("Making GET request");
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            error!("HTTP request failed with status: {}", response.status());
            return Err(anyhow::anyhow!("HTTP request failed: {}", response.status()));
        }
        
        Ok(response)
    }

    #[instrument(skip(self), fields(url = %url))]
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let response = self.get(url).await?;
        let json = response.json::<T>().await?;
        Ok(json)
    }

    #[instrument(skip(self, body), fields(url = %url))]
    pub async fn post_json<T: DeserializeOwned, B: serde::Serialize>(&self, url: &str, body: &B) -> Result<T> {
        debug!("Making POST request");
        let response = self.client
            .post(url)
            .json(body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            error!("HTTP POST failed with status: {}", response.status());
            return Err(anyhow::anyhow!("HTTP POST failed: {}", response.status()));
        }
        
        let json = response.json::<T>().await?;
        Ok(json)
    }

    #[instrument(skip(self), fields(url = %url))]
    pub async fn delete(&self, url: &str) -> Result<()> {
        debug!("Making DELETE request");
        let response = self.client.delete(url).send().await?;
        
        if !response.status().is_success() {
            error!("HTTP DELETE failed with status: {}", response.status());
            return Err(anyhow::anyhow!("HTTP DELETE failed: {}", response.status()));
        }
        
        Ok(())
    }

    pub fn request(&self, method: reqwest::Method, url: &str) -> RequestBuilder {
        self.client.request(method, url)
    }
}