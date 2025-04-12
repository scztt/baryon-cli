mod data {

    use baryon::core::repository::Repository;
    use baryon::mocks::repository::MockRepository;

    #[tokio::test]
    async fn can_load_repository() {
        let repo = MockRepository::new().await;

        let mut package_names = repo
            .get_packages()
            .iter()
            .map(|package| package.name.clone())
            .collect::<Vec<String>>();

        package_names.sort();

        assert_eq!(package_names, vec!["package1", "package2", "package3"]);
    }

    #[tokio::test]
    async fn can_get_individual_packages() {
        let repo = MockRepository::new().await;

        let package1 = repo.get_package("package1").unwrap();
        assert_eq!(package1.name, "package1");

        let package3 = repo.get_package("package3").unwrap();
        assert_eq!(package3.name, "package3");
    }

    #[tokio::test]
    async fn can_get_depedencies() {
        let repo = MockRepository::new().await;

        let package1 = repo.get_package("package1").unwrap();
        let version1 = package1.releases[0].clone();
        let mut dep_names = version1
            .dependencies
            .iter()
            .map(|dep| dep.0.to_string())
            .collect::<Vec<String>>();
        dep_names.sort();

        assert_eq!(dep_names, vec!["package2", "package3"]);
    }
}

mod resolution {
    use baryon::core::dependencies::{PackageRequirement, PackageResolver, Repository, Strategy};
    use baryon::core::repository::Repository as Repo;
    use baryon::mocks::repository::MockRepository;

    #[tokio::test]
    async fn test_package_resolution() {
        let mock = MockRepository::new().await;
        let repo = Repository::new(mock.get_packages());

        let strategy = Strategy::new();
        let requirements =
            vec![PackageRequirement::new("package1".to_string(), "1.0.0".to_string()).unwrap()];

        let mut resolver = PackageResolver::new(requirements, repo, strategy);
        let resolution = resolver.resolve();

        assert!(resolution.is_ok());
        resolution
            .map(|resolved_packages| {
                assert_eq!(resolved_packages.len(), 3);
                for (package_name, version) in resolved_packages.iter() {
                    println!("   Package: {} -> {}", package_name, version.version);
                }
            })
            .unwrap_or_default()
    }
}
