use std::collections::HashMap;

use crate::types::{PVData, Error, Config, Meta};
use crate::decode_helpers::format_date_for_archiver;
use crate::decode::decode_response;
use url::Url;
use reqwest::Client;

pub struct ArchiverClient {
    client: Client,
    base_url: String,
}

impl ArchiverClient {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_secs))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: config.url,
        }
    }

    pub async fn fetch_data(&self, pv: &str, start: i64, end: i64) -> Result<PVData, Error> {
        let url = self.build_url(
            "data/getData.raw",
            &[
                ("pv", pv),
                ("from", &format_date_for_archiver(start * 1000).ok_or_else(|| Error::Invalid("Invalid start timestamp".to_string()))?),
                ("to", &format_date_for_archiver(end * 1000).ok_or_else(|| Error::Invalid("Invalid end timestamp".to_string()))?),
                ("fetchLatestMetadata", "true"),
            ],
        )?;
    
        let response = self.client
            .get(url.clone())
            .send()
            .await
            .map_err(Error::Network)?;

        if !response.status().is_success() {
            return Err(Error::Invalid(format!(
                "Server returned {} for {}",
                response.status(), url
            )));
        }
    
        let bytes = response.bytes().await.map_err(Error::Network)?;
        let pv_data = decode_response(bytes)?;
        
        pv_data.into_iter().next().ok_or_else(|| Error::Invalid("No data returned".to_string()))
    }

    pub async fn get_metadata(&self, pv: &str) -> Result<Meta, Error> {
        let url = self.build_url(
            "getMetadata",
            &[("pv", pv)],
        )?;

        let response = self.client
            .get(url.clone())
            .send()
            .await
            .map_err(Error::Network)?;

        if !response.status().is_success() {
            return Err(Error::Invalid(format!(
                "Server returned {} for {}",
                response.status(), url
            )));
        }

        let meta_data: HashMap<String, String> = response.json().await.map_err(Error::Network)?;
        Ok(Meta(meta_data))
    }

    fn build_url(&self, path: &str, params: &[(&str, &str)]) -> Result<Url, Error> {
        let mut url = Url::parse(&format!("{}/{}", self.base_url, path))
            .map_err(|e| Error::Invalid(format!("Invalid URL: {}", e)))?;
        url.query_pairs_mut().extend_pairs(params);
        Ok(url)
    }
}