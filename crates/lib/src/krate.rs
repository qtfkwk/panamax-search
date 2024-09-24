use {
    crate::functions::path_parent,
    anyhow::{anyhow, Result},
    flate2::read::GzDecoder,
    log::*,
    rev_lines::RevLines,
    semver::Version,
    serde::{Deserialize, Serialize},
    std::{
        fs::File,
        io::Read,
        path::{Path, PathBuf},
    },
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate {
    #[serde(skip)]
    pub name: String,

    #[serde(rename = "d", skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "v", skip_serializing_if = "Option::is_none")]
    pub latest_ny: Option<Version>,

    #[serde(rename = "y", skip_serializing_if = "Option::is_none")]
    pub latest: Option<Version>,
}

impl Crate {
    pub fn new(index_file: &Path) -> Result<Crate> {
        debug!("{index_file:?}");

        let mut name = None;
        let mut latest_ny = None;
        let mut latest = None;

        match File::open(index_file) {
            Ok(f) => {
                // Parse lines of the crate's index file from the bottom up
                for line in RevLines::new(f).flatten() {
                    // Deserialize line as `CrateIndex`
                    match serde_json::from_str::<CrateIndex>(&line) {
                        Ok(i) => {
                            // Set name once
                            if name.is_none() {
                                name = Some(i.name.clone());
                            }

                            if i.yanked {
                                if latest.is_none() {
                                    latest = Some(i.vers.clone());
                                }
                            } else {
                                if latest_ny.is_none() {
                                    latest_ny = Some(i.vers.clone());
                                }
                                break;
                            }
                        }
                        Err(e) => {
                            return Err(anyhow!(
                                "{index_file:?}: Deserialization errror: {e}; line = {line:?}"
                            ));
                        }
                    }
                }

                if name.is_none() {
                    return Err(anyhow!("{index_file:?}: No name"));
                }

                if latest.is_none() && latest_ny.is_none() {
                    return Err(anyhow!(
                        "{index_file:?}: No latest or latest non-yanked version"
                    ));
                }

                Ok(Crate {
                    name: name.unwrap(),
                    description: None,
                    latest_ny,
                    latest,
                })
            }
            Err(e) => Err(anyhow!("{index_file:?}: Could not open file: {e}")),
        }
    }

    pub fn add_description(&mut self, index_file: &Path) {
        let (crate_file, version) = self.crate_file_and_version(index_file);

        match self.get_cargo_toml(&crate_file, &version) {
            Ok(content) => {
                // Try to deserialize with a `package` section
                match toml::from_str::<CargoTomlPackage>(&content) {
                    Ok(t) => {
                        if let Some(d) = t.package.description {
                            self.description = Some(d.clone());
                        } else {
                            debug!("{crate_file:?}: No package.description");
                        }
                    }
                    Err(_e) => {
                        // Try to deserialize with a `project` section
                        match toml::from_str::<CargoTomlProject>(&content) {
                            Ok(t) => {
                                debug!("{crate_file:?}: Has project section");
                                if let Some(d) = t.project.description {
                                    self.description = Some(d.clone());
                                } else {
                                    debug!("{crate_file:?}: No project.description");
                                }
                            }
                            Err(e) => {
                                // Failed to deserialize
                                debug!("{crate_file:?}: Deserialization error: {e:?}");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("{crate_file:?}: {e}");
            }
        }
    }

    fn crate_file_and_version(&self, index_file: &Path) -> (PathBuf, String) {
        // Use the latest non-yanked version if possible, otherwise use the latest yanked version
        let version = if let Some(latest_ny) = &self.latest_ny {
            latest_ny.to_string()
        } else if let Some(latest) = &self.latest {
            latest.to_string()
        } else {
            unreachable!()
        };

        // Convert the index file path into the related crate file path
        let crate_file = match self.name.len() {
            // `mirror/crates/1/a/0.0.0/a-0.0.0.crate`
            1 => path_parent(index_file, 3)
                .join("crates")
                .join("1")
                .join(&self.name[..1]),

            // `mirror/crates/2/aa/0.0.0/aa-0.0.0.crate`
            2 => path_parent(index_file, 3)
                .join("crates")
                .join("2")
                .join(&self.name[..2]),

            // `mirror/crates/3/a/aaa/0.0.0/a-0.0.0.crate`
            3 => path_parent(index_file, 4)
                .join("crates")
                .join("3")
                .join(&self.name[..1]),

            // `mirror/crates/aa/aa/0.0.0/aaaa-0.0.0.crate`
            _ => path_parent(index_file, 4)
                .join("crates")
                .join(&self.name[..2])
                .join(&self.name[2..4]),
        }
        .join(&self.name)
        .join(&version)
        .join(format!("{}-{version}.crate", self.name));

        (crate_file, version)
    }

    fn get_cargo_toml(&self, crate_file: &Path, version: &str) -> Result<String> {
        let file = File::open(crate_file)?;
        let decoder = GzDecoder::new(file);
        let mut r = tar::Archive::new(decoder);

        for entry in r.entries()? {
            match entry {
                Ok(mut entry) => {
                    let path = entry.path()?;
                    let path = path.to_str().unwrap();
                    for filename in ["Cargo.toml", "cargo.toml"] {
                        if path == format!("{}-{version}/{filename}", self.name) {
                            if filename == "cargo.toml" {
                                debug!("{crate_file:?}: Has cargo.toml");
                            }
                            let mut s = String::new();
                            entry.read_to_string(&mut s)?;
                            return Ok(s);
                        }
                    }
                }
                _ => continue,
            }
        }

        Err(anyhow!("No Cargo.toml"))
    }
}

#[derive(Deserialize)]
struct CargoTomlPackage {
    package: Package,
}

#[derive(Deserialize)]
struct CargoTomlProject {
    project: Package,
}

#[derive(Deserialize)]
struct Package {
    description: Option<String>,
}

#[derive(Deserialize)]
struct CrateIndex {
    name: String,
    vers: Version,
    yanked: bool,
}
