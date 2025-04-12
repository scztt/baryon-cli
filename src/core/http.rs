use miette::Diagnostic;
use reqwest::Client;
use serde_json as json;
use std::{
    io::Write,
    time::{Duration, SystemTime},
};
use thiserror::Error;

pub struct CacheSettings {
    pub cache_path: String,
    pub cache_timeout: Duration,
}

struct CachedValue<T> {
    value: T,
    time: SystemTime,
}

pub struct Query {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
}

pub struct RemoteEndpoint<T> {
    query: Query,

    cache_path: String,
    cache_timeout: Duration,
    cache: Option<CachedValue<T>>,
}

#[derive(Debug, Error, Diagnostic)]
pub enum EndpointError {
    #[error("Failed to parse JSON: {0}")]
    Json(#[from] json::Error),

    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Network request failed: {0}")]
    IO(#[from] std::io::Error),
}

impl<T> RemoteEndpoint<T>
where
    T: for<'de> serde::de::Deserialize<'de>,
{
    pub fn new(settings: &CacheSettings, query: Query) -> Self {
        Self {
            query,
            cache_path: settings.cache_path.clone(),
            cache_timeout: settings.cache_timeout,
            cache: None,
        }
    }

    async fn load_from_disk(&mut self) -> Result<(), EndpointError> {
        if self.cache.is_some() {
            return Ok(());
        }

        let maybe_data = std::fs::read_to_string(&self.cache_path);
        let maybe_mod_time = std::fs::metadata(&self.cache_path).and_then(|meta| meta.modified());

        if let (Ok(data), Ok(mod_time)) = (maybe_data, maybe_mod_time) {
            let value = serde_json::from_str::<T>(&data)?;
            self.cache = Some(CachedValue {
                value,
                time: mod_time,
            });
            return Ok(());
        }

        self.cache = None;
        Ok(()) // @TODO filesystem error here is weird, but not fatal - what should we do?
    }

    async fn load_from_remote(&mut self) -> Result<(), EndpointError> {
        let client = Client::new();
        let response = client.get(&self.query.url).send().await?;

        if response.status().is_success() {
            let cache_time = response.headers().get("Last-Modified").and_then(|v| {
                let time_string = v.to_str().ok()?;
                if let Ok(time) = time_string.parse::<i64>() {
                    return Some(SystemTime::UNIX_EPOCH + Duration::from_secs(time as u64));
                }
                None
            });
            let text = response.text().await?;
            let value = serde_json::from_str::<T>(&text)?;

            let mut file = std::fs::File::options()
                .write(true)
                .open(&self.cache_path)?;
            file.write_all(text.as_bytes())?;
            file.set_modified(cache_time.unwrap_or(SystemTime::now()))?;

            self.cache = Some(CachedValue {
                value,
                time: cache_time.unwrap_or(SystemTime::now()),
            });
        } else {
            return Err(EndpointError::Network(
                response.error_for_status().unwrap_err(),
            ));
        }

        Ok(())
    }

    pub async fn data(&mut self) -> Result<&T, EndpointError>
    where
        T: serde::de::DeserializeOwned,
    {
        self.load_from_disk().await?;

        let cache_expired = self
            .cache
            .as_ref()
            .map(|cache| cache.time.elapsed().unwrap_or_default() > self.cache_timeout)
            .unwrap_or(true);

        if cache_expired {
            self.load_from_remote().await?;
        }

        assert!(self.cache.is_some(), "Cache should be loaded");

        Ok(&self.cache.as_ref().unwrap().value)
    }
}
