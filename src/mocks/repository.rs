use crate::core::repository::Repository;
use crate::specs::{Package, Repository as RepositorySpec};
use serde_yaml;
use tokio::fs;
use tokio::io::AsyncReadExt;

pub struct MockRepository {
    pub packages: RepositorySpec,
}

impl MockRepository {
    pub async fn new() -> Self {
        let mut dst = String::new();
        fs::File::open("src/mocks/repository.yaml")
            .await
            .unwrap()
            .read_to_string(&mut dst)
            .await
            .unwrap();

        Self {
            packages: serde_yaml::from_str(&dst).unwrap(),
        }
    }
}

impl Repository for MockRepository {
    fn get_package(&self, package_name: &str) -> Option<&Package> {
        self.packages.get(package_name)
    }

    fn get_packages(&self) -> Vec<&Package> {
        self.packages.values().collect()
    }
}
