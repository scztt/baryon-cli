use std::collections::HashMap;

use crate::core::http::{Query, RemoteEndpoint};
use crate::specs::{self, Package, Repository as RepositoryDesc};

use super::settings::Settings;

pub struct HTTPRepository {
    repo_endpoint: RemoteEndpoint<RepositoryDesc>,
    desc: RepositoryDesc,
}

pub trait Repository {
    fn get_packages(&self) -> Vec<&Package>;
    fn get_package(&self, package_name: &str) -> Option<&Package>;
}

impl HTTPRepository {
    pub fn new(settings: &Settings) -> Self {
        let repo_endpoint = RemoteEndpoint::new(
            &settings.cache_settings,
            Query {
                url: settings.repository_url.clone(),
                method: "POST".to_string(),
                headers: vec![],
            },
        );

        Self {
            repo_endpoint,
            desc: specs::Repository(HashMap::new()),
        }
    }
}

impl Repository for HTTPRepository {
    fn get_package(&self, package_name: &str) -> Option<&Package> {
        self.desc.get(package_name)
    }
    fn get_packages(&self) -> Vec<&Package> {
        self.desc.iter().map(|(_, package)| package).collect()
    }
}
