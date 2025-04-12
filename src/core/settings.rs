use crate::core::http::CacheSettings;

pub struct Settings {
    pub global_repository_path: String,
    pub repository_url: String,
    pub cache_settings: CacheSettings,
}
