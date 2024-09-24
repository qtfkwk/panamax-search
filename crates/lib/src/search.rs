use {
    crate::krate::Crate,
    owo_colors::OwoColorize,
    rayon::prelude::*,
    regex::{Regex, RegexBuilder, RegexSetBuilder},
    semver::Version,
    std::collections::{BTreeMap, HashSet},
};

pub struct Search {
    pub name_exact: Vec<Crate>,
    pub name_contains: Vec<Crate>,
    pub desc_contains: Vec<Crate>,
    re: Vec<Regex>,
}

impl Search {
    pub fn new(
        queries: &[String],
        case_insensitive: bool,
        crates: &BTreeMap<String, Crate>,
    ) -> Search {
        let mut names = HashSet::new();

        let mut name_exact = vec![];
        for query in queries {
            if let Some(crate_) = crates.get(query) {
                names.insert(crate_.name.clone());
                name_exact.push(crate_.clone());
            }
        }

        // Create a RegexSet with all queries
        let re = RegexSetBuilder::new(queries)
            .case_insensitive(case_insensitive)
            .build()
            .unwrap();

        // Filter matching names
        let name_contains = crates
            .par_iter()
            .filter_map(|(name, crate_)| {
                if !names.contains(name) && re.is_match(name) {
                    Some(crate_.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Update the set of matched crates
        for crate_ in &name_contains {
            names.insert(crate_.name.clone());
        }

        // Filter matching descriptions
        let desc_contains = crates
            .par_iter()
            .filter_map(|(name, crate_)| {
                if names.contains(name) {
                    None
                } else if let Some(description) = &crate_.description {
                    if re.is_match(description) {
                        Some(crate_.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Convert the RegexSet to equivalent Vec<Regex>
        let re = re
            .patterns()
            .iter()
            .map(|x| {
                RegexBuilder::new(x)
                    .case_insensitive(case_insensitive)
                    .build()
                    .unwrap()
            })
            .collect::<Vec<_>>();

        // Return the search results
        Search {
            name_exact,
            name_contains,
            desc_contains,
            re,
        }
    }

    pub fn to_string(&self, include_yanked: bool, highlight_matches: bool) -> String {
        let mut width = 0;
        let mut lines = vec![];

        // Collate results in order from each category and measure the widest name and version
        for v in [&self.name_exact, &self.name_contains, &self.desc_contains] {
            for crate_ in v {
                let version = if include_yanked && crate_.latest.is_some() {
                    crate_.latest.as_ref().unwrap().to_string()
                } else {
                    crate_
                        .latest_ny
                        .as_ref()
                        .unwrap_or(&Version::new(0, 0, 0))
                        .to_string()
                    // Latest non-yanked version or "0.0.0" if all versions were yanked;
                    // this matches `cargo search` behavior.
                };

                let name_and_version = format!("{} = \"{version}\"    ", crate_.name);

                width = width.max(name_and_version.len());

                lines.push((name_and_version, &crate_.description));
            }
        }

        // Build result string
        lines
            .par_iter()
            .map(|(name_and_version, description)| {
                if let Some(d) = description {
                    let (nv, d) = if highlight_matches {
                        (
                            &self.highlight(name_and_version),
                            &self.highlight(&d.replace("\n", "\\n").replace("\r", "\\r")),
                        )
                    } else {
                        (name_and_version, d)
                    };
                    let s = " ".repeat(width - name_and_version.len());
                    format!("{nv}{s}# {d}\n")
                } else if highlight_matches {
                    format!("{}\n", &self.highlight(name_and_version))
                } else {
                    format!("{name_and_version}\n")
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn highlight(&self, s: &str) -> String {
        let mut r = s.to_string();
        for re in &self.re {
            for m in re.find_iter(s).map(|m| m.as_str()) {
                r = r.replace(m, &m.green().bold().to_string());
            }
        }
        r
    }

    pub fn to_vec(&self) -> Vec<Crate> {
        self.name_exact
            .iter()
            .chain(self.name_contains.iter())
            .chain(self.desc_contains.iter())
            .cloned()
            .collect()
    }
}
