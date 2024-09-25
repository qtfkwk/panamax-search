# About

Search crates in a panamax mirror like `cargo search`

- Parses each crate name, latest version, and latest non-yanked version from its index file
- Extracts each crate's description from its crate file
- Saves to and restores from a cache file
- Updates the cache file on first use following the mirror being sync'd
- Searches can include one or more queries and be either case sensitive or not
- Search results are categorized by search relevance (exact name match, name contains, or
  description contains)
- Search results can be formatted like `cargo search` output

# Notes

1. A relatively small percentage of crates have issues getting the description from the `Cargo.toml`
   file embedded in the `.crate` file for various reasons; some of these can be reasonably
   mitigated, others force `panamax-search` to omit the description.

    Affected | Issue | Status
    ---|---|---
    *275 crates* | No `description` | Omit
    *27 crates* | Has `cargo.toml` instead of `Cargo.toml` | Fixed
    *13 crates* | Has `[project]` instead of `[package]` | Fixed
    `pnetlink` | Two `[dependencies]` sections | N/A
    `screen-13` | Corrupt `.crate` file | N/A
    `servo` | Zero entries in `.crate` file | Omit
    `svd_macros` | Malformed `Cargo.toml` | Omit

   N/A (not applicable) means that the issue was observed in older versions of these crates, but has
   since been fixed by its author.

2. JSON was chosen as the cache file format for a few reasons.
   TOML was a leading contender for consistency, but unfortunately it isn't extremely conducive to
   BTreeMap data; in particular, it puts each crate in a `[crates.name]` section with `version` and
   `description` fields, which makes it less than the ideal (1 crate per line) and impossible to
   usefully deserialize the `name` field (?).
   JSON doesn't have these problems and can also be easily queried by both native web as well as CLI
   tools like `jq`.

3. By default, the versions displayed are the latest non-yanked version of each crate, however the
   library also captures and uses the actual latest version ignoring yanked status.

   The cache file stores either just the latest version (`v`) if there are no yanked versions, just
   the latest yanked version (`y`) if there are no non-yanked versions, or both if present.

   Capturing separate descriptions for each latest and latest non-yanked version was contemplated,
   but there were zero observed instances where the descriptions were different.
   Storing duplicate descriptions nearly doubled the cache file size for no actual gain, and even
   storing as a `None` (just in case the need ever arose) still added about 25%.
   So the idea was tabled.

