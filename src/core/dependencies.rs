use crate::specs::Package;
use semver::Version;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct PackageVersion {
    pub name: String,
    pub version: Version,
    pub required_by: Vec<PackageRequirement>, // Names of packages that require this one
}

impl PackageVersion {
    fn new(name: &str, version_str: &str) -> Result<Self, semver::Error> {
        Ok(Self {
            name: name.to_string(),
            version: Version::parse(version_str)?,
            required_by: Vec::new(),
        })
    }
}

use semver::VersionReq;

#[derive(Debug, Clone)]
pub struct PackageRequirement {
    pub name: String,
    pub spec: VersionReq,
    pub required_by: Vec<PackageRequirement>, // Names of packages that require this one
}

impl PackageRequirement {
    pub fn new(name: String, spec_str: String) -> Result<Self, semver::Error> {
        Ok(Self {
            name: name.to_string(),
            spec: VersionReq::parse(&spec_str)?,
            required_by: Vec::new(),
        })
    }

    pub fn matches(&self, version: &Version) -> bool {
        self.spec.matches(version)
    }
}

#[derive(Debug)]
pub struct Repository {
    pub data: HashMap<String, HashMap<Version, Vec<PackageRequirement>>>,
}

impl Repository {
    pub fn new(repo: Vec<&Package>) -> Self {
        let mut data = HashMap::new();
        for package in repo.iter() {
            let mut releases = HashMap::new();
            for item in package.releases.iter() {
                let version = Version::parse(&item.version).unwrap();
                let dependencies = item
                    .dependencies
                    .iter()
                    .map(|dep| {
                        PackageRequirement::new(dep.0.to_string(), dep.1.to_string()).unwrap()
                    })
                    .collect::<Vec<_>>();

                releases.insert(version, dependencies);
            }
            data.insert(package.name.clone(), releases);
        }
        Self { data }
    }

    fn get_versions(&self, package: &str) -> Vec<PackageVersion> {
        self.data
            .get(package)
            .map(|versions| {
                versions
                    .keys()
                    .cloned()
                    .map(|v| PackageVersion {
                        name: package.to_string(),
                        version: v,
                        required_by: Vec::new(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn get_dependencies(&self, package_version: &PackageVersion) -> Vec<PackageRequirement> {
        self.data
            .get(&package_version.name)
            .and_then(|versions| versions.get(&package_version.version))
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct Strategy {
    conservative: bool,
    avoid_prerelease: bool,
}

impl Default for Strategy {
    fn default() -> Self {
        Self {
            conservative: true,
            avoid_prerelease: false,
        }
    }
}

impl Strategy {
    pub fn new() -> Self {
        Self {
            conservative: true,
            avoid_prerelease: false,
        }
    }

    fn filter_versions(&self, versions: Vec<PackageVersion>) -> Vec<PackageVersion> {
        versions
            .into_iter()
            .filter(|v| {
                if self.avoid_prerelease {
                    v.version.pre.is_empty()
                } else {
                    true
                }
            })
            .collect()
    }

    fn sort_versions(&self, mut versions: Vec<PackageVersion>) -> Vec<PackageVersion> {
        if self.conservative {
            versions.sort_by(|a, b| b.version.cmp(&a.version));
        } else {
            versions.sort_by(|a, b| a.version.cmp(&b.version));
        }
        versions
    }
}

#[derive(Debug, Clone)]
struct State {
    requirements: Vec<PackageRequirement>,
    selected: HashMap<String, PackageVersion>,
    current_requirement: PackageRequirement,
    possibilities: Vec<PackageVersion>,
    depth: usize,
    name: String,
}

pub struct FailedRequirement {
    pub desc: String,
}

pub struct PackageResolver {
    initial_requirements: Vec<PackageRequirement>,
    repo: Repository,
    strategy: Strategy,
    states: Vec<State>,
    selected: HashMap<String, PackageVersion>,
    requirements: Vec<PackageRequirement>,
    errors: HashMap<String, (Option<semver::Version>, semver::VersionReq)>,
    depth: usize,
    start_time: Instant,
}

impl PackageResolver {
    pub fn new(
        requirements: Vec<PackageRequirement>,
        repo: Repository,
        strategy: Strategy,
    ) -> Self {
        PackageResolver {
            initial_requirements: requirements.clone(),
            repo,
            strategy,
            states: Vec::new(),
            selected: HashMap::new(),
            requirements,
            errors: HashMap::new(),
            depth: 0,
            start_time: Instant::now(),
        }
    }

    pub fn resolve(&mut self) -> Result<&HashMap<String, PackageVersion>, FailedRequirement> {
        while let Some(current_req) = self.requirements.pop() {
            // We've already selected a version for this package
            let existing = self.selected.get(&current_req.name);
            if let Some(existing_version) = existing {
                if current_req.matches(&existing_version.version) {
                    // %s compatible with current version %s
                    self.errors.remove(&current_req.name);
                } else {
                    // %s conflicts with existing version %s
                    self.errors.insert(
                        current_req.name.clone(),
                        (
                            Some(existing_version.version.clone()),
                            current_req.spec.clone(),
                        ),
                    );

                    let parent = current_req.required_by.last().cloned();
                    let parent_state = self.find_state(parent.as_ref());
                    let conflict_parent =
                        if parent_state.is_some_and(|s| !s.possibilities.is_empty()) {
                            // parent of current requirement has other possibilities, so try this
                            self.find_conflict_parent(&current_req, None)
                        } else {
                            // try to handle by stepping back both the current requirement and the existing requirement
                            let existing_parent = existing_version.required_by.last();
                            self.find_conflict_parent(&current_req, existing_parent)
                        };

                    if let Some(parent_req) = conflict_parent {
                        self.resolve_conflict(&parent_req)?;
                    } else {
                        return Err(FailedRequirement {
                            desc: current_req.name.clone(),
                        });
                    }
                }
            } else {
                // We haven't yet selected a version for this package, so lets do it.
                let versions = self.repo.get_versions(&current_req.name);
                let compatible_versions_spec: Vec<_> = versions
                    .iter()
                    .filter(|v| current_req.matches(&v.version))
                    .cloned()
                    .collect();

                let mut compatible_versions = self
                    .strategy
                    .filter_versions(compatible_versions_spec.clone());

                compatible_versions = self.strategy.sort_versions(compatible_versions);

                if compatible_versions.is_empty() {
                    // No compatible versions for %s %s.
                    self.errors
                        .insert(current_req.name.clone(), (None, current_req.spec.clone()));
                    if current_req.required_by.is_empty() {
                        // Can't match a top level package. Try upgrading to a newer version.
                        return Err(FailedRequirement {
                            desc: current_req.name.clone(),
                        });
                    } else {
                        let parent = self.find_conflict_parent(&current_req, None);
                        if let Some(parent_req) = parent {
                            self.resolve_conflict(&parent_req)?;
                        } else {
                            return Err(FailedRequirement {
                                desc: current_req.name.clone(),
                            });
                        }
                    }
                } else {
                    // Found %s versions for %s that match spec.
                    let mut state = State {
                        requirements: self.requirements.clone(),
                        selected: self.selected.clone(),
                        current_requirement: current_req.clone(),
                        possibilities: compatible_versions,
                        depth: self.depth,
                        name: current_req.name.clone(),
                    };
                    let selected_version = state.possibilities.pop().unwrap();
                    self.select_package(&selected_version, &current_req);
                    self.states.push(state);
                }
            }
        }

        Ok(&self.selected)
    }

    fn select_package(
        &mut self,
        package_version: &PackageVersion,
        current_req: &PackageRequirement,
    ) {
        // Selecting version %s
        let mut pv = package_version.clone();
        pv.required_by = current_req.required_by.clone();
        pv.required_by.push(current_req.clone());
        self.selected.insert(pv.name.clone(), pv.clone());

        // Adding dependencies for package %s: %s
        let mut dependencies = self.repo.get_dependencies(&pv);
        for dep in dependencies.iter_mut() {
            dep.required_by = current_req.required_by.clone();
            dep.required_by.push(current_req.clone());
        }
        self.requirements.extend(dependencies);
    }

    fn find_conflict_parent(
        &self,
        current_req: &PackageRequirement,
        existing_req: Option<&PackageRequirement>,
    ) -> Option<PackageRequirement> {
        let mut current = Some(current_req.clone());
        let mut existing = existing_req.cloned();

        while current.is_some() || existing.is_some() {
            // If there were other possible version choices in either the current or the already selected
            // package, return one of those
            if let Some(ref cur) = current {
                if let Some(state) = self.find_state(Some(cur)) {
                    if !state.possibilities.is_empty() {
                        return Some(cur.clone());
                    }
                }
            }
            if let Some(ref ex) = existing {
                if let Some(state) = self.find_state(Some(ex)) {
                    if !state.possibilities.is_empty() {
                        return Some(ex.clone());
                    }
                }
            }

            // If not, we need to walk one level back up the requirements chain and check again
            current = current.and_then(|c| c.required_by.last().cloned());
            existing = existing.and_then(|e| e.required_by.last().cloned());
        }

        // Requirement for %s and existing requirement %s conflict, and no possible resolution can be found.
        None
    }

    fn find_state(&self, requirement: Option<&PackageRequirement>) -> Option<&State> {
        if let Some(req) = requirement {
            for state in &self.states {
                if state.name == req.name {
                    return Some(state);
                }
            }
        }
        None
    }

    fn reset_state(&mut self, state: &State) {
        if state.requirements.is_empty() && state.selected.is_empty() {
            // Null state!
            panic!("Null state!");
        }

        // roll states stack back to this state
        while let Some(popped) = self.states.pop() {
            if popped.name == state.name {
                break;
            }
        }

        self.requirements = state.requirements.clone();
        self.selected = state.selected.clone();
        self.depth = state.depth;
    }

    fn resolve_conflict(
        &mut self,
        requirement: &PackageRequirement,
    ) -> Result<(), FailedRequirement> {
        let maybe_state = self.find_state(Some(requirement)).cloned();

        // Rewinding to conflict state %s to find a new dependency resolution.
        if let Some(mut state) = maybe_state {
            if let Some(selected_version) = state.possibilities.pop() {
                let current_requirement = state.current_requirement.clone();
                self.reset_state(&state);
                self.select_package(&selected_version, &current_requirement);

                // If state has other possible versions remaining, keep it around
                if !state.possibilities.is_empty() {
                    self.states.push(state);
                }
                Ok(())
            } else {
                Err(FailedRequirement {
                    desc: requirement.name.clone(),
                })
            }
        } else {
            Err(FailedRequirement {
                desc: requirement.name.clone(),
            })
        }
    }
}
