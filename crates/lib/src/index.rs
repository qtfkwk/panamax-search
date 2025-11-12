use {
    crate::{
        functions::{ensure_directory, filter_entries},
        krate::Crate,
        search::Search,
    },
    anyhow::{Result, anyhow},
    log::{error, info, trace},
    rayon::prelude::*,
    serde::{
        Deserialize,
        de::{Deserializer, MapAccess, Visitor},
    },
    std::{
        collections::BTreeMap,
        fs::{File, read_to_string},
        io::{BufWriter, Write},
        path::Path,
    },
    walkdir::WalkDir,
};

pub struct Index(BTreeMap<String, Crate>);

impl Index {
    /**
    # Errors

    Returns an error if it is not able to load the index from either the cache file or the mirror
    directory
    */
    pub fn load(mirror_directory: &Path) -> Result<Index> {
        if let Ok(index) = Index::load_from_cache_file(mirror_directory) {
            Ok(index)
        } else {
            Index::load_from_mirror_directory(mirror_directory)
        }
    }

    /**
    # Errors

    Returns an error if it is not able to load the index from the cache file
    */
    pub fn load_from_cache_file(mirror_directory: &Path) -> Result<Index> {
        ensure_directory(mirror_directory)?;

        let cache_file = mirror_directory.join("search.json");
        let config_file = mirror_directory.join("crates.io-index").join("config.json");

        let cache_file_s = cache_file.display().to_string();

        if cache_file.is_file() && config_file.is_file() {
            if cache_file.metadata()?.modified()? > config_file.metadata()?.modified()? {
                info!("Load index from cache file {cache_file_s:?}");

                return match read_to_string(&cache_file) {
                    Ok(s) => Index::from_json(&s),
                    Err(e) => {
                        error!("Could not read cache file {cache_file_s:?}: {e}");
                        Err(anyhow!("Could not read cache file {cache_file_s:?}: {e}"))
                    }
                };
            }

            info!("Cache file is old {cache_file_s:?}");
            return Err(anyhow!("Cache file is old {cache_file_s:?}"));
        }

        Err(anyhow!(
            "Cannot load index from cache file {cache_file_s:?}",
        ))
    }

    /**
    # Panics

    Panics if not able to load a crate from the index file

    # Errors

    Returns an error is not able to save the cache file
    */
    pub fn load_from_mirror_directory(mirror_directory: &Path) -> Result<Index> {
        ensure_directory(mirror_directory)?;

        let mirror_directory_s = mirror_directory.display().to_string();

        info!("Load index from mirror directory {mirror_directory_s:?}");
        let index = Index(
            WalkDir::new(mirror_directory.join("crates.io-index"))
                .sort_by_file_name()
                .into_iter()
                .filter_entry(filter_entries)
                .flatten()
                .collect::<Vec<_>>()
                .into_par_iter()
                .filter_map(|x| {
                    let index_file = x.path();
                    index_file.is_file().then_some(index_file.to_path_buf())
                })
                .map(|index_file| {
                    let mut crate_ = Crate::new(&index_file).unwrap();
                    trace!("{crate_:?}");
                    crate_.add_description(&index_file);
                    (crate_.name.clone(), crate_)
                })
                .collect(),
        );

        let cache_file = mirror_directory.join("search.json");
        index.save(&cache_file)?;

        Ok(index)
    }

    fn save(&self, cache_file: &Path) -> Result<()> {
        let cache_file_s = cache_file.display().to_string();
        info!("Save cache file {cache_file_s:?}");
        Ok(BufWriter::new(File::create(cache_file)?).write_all(self.to_json()?.as_bytes())?)
    }

    #[must_use]
    pub fn search(&self, queries: &[String], case_insensitive: bool) -> Search {
        Search::new(queries, case_insensitive, &self.0)
    }

    /**
    Custom JSON serializer enabling one entry per line

    ```text
    {"name-a":{"d":"Description","v":"0.0.0","y":"0.0.0"},
    "name-b":{"d":"Description","v":"0.0.0","y":"0.0.0"},
    "name-z":{"d":"Description","v":"0.0.0","y":"0.0.0"}}
    ```
    */
    fn to_json(&self) -> Result<String> {
        let r = self
            .0
            .par_iter()
            .map(|(name, value)| (name, serde_json::to_string(value)))
            .collect::<Vec<_>>();

        let errors = r
            .par_iter()
            .filter(|(_name, value)| value.is_err())
            .collect::<Vec<_>>();

        if errors.is_empty() {
            Ok(format!(
                "{{{}}}",
                r.par_iter()
                    .map(|(name, value)| format!("\"{name}\":{}", value.as_ref().unwrap()))
                    .collect::<Vec<_>>()
                    .join(",\n")
            ))
        } else {
            Err(anyhow!("Serialization to JSON failed: {errors:?}"))
        }
    }

    fn from_json(s: &str) -> Result<Index> {
        Ok(serde_json::from_str::<Index>(s)?)
    }
}

impl<'de> Deserialize<'de> for Index {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(IndexVisitor {})
    }
}

struct IndexVisitor;

impl<'de> Visitor<'de> for IndexVisitor {
    type Value = Index;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("name:{description,latest_ny,latest}")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        // Custom deserializer that sets each crate's name from the key

        let mut crates = BTreeMap::new();

        while let Some((name, mut crate_)) = access.next_entry::<String, Crate>()? {
            crate_.name.clone_from(&name);
            crates.insert(name, crate_);
        }

        Ok(Index(crates))
    }
}
