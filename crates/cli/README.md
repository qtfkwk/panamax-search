# About

Search crates in a [panamax] mirror like `cargo search`

*Prerequisite: Create a [panamax] mirror at `~/panamax`.*

- `panamax-search -P` parses the [panamax] mirror at `~/panamax`, or the path given by the `-m` option,
  and creates/updates a cache file at `~/panamax/search.json`.

  *This step enables subsequent searches to load the cache file instead of reparsing the mirror.*

- Search for crates with `blah` in their name or description:
  `panamax-search blah`

  *Consider using `-s` and/or `-y` options with search commands to enable case sensitive searching
  or including yanked versions, respectively.*

See also:

* `panamax-search-lib`: Library crate
  ([Crates.io](https://crates.io/crates/panamax-search-lib),
  [GitHub](https://github.com/qtfkwk/panamax-search/tree/main/crates/lib))

[panamax]: https://crates.io/crates/panamax

# Notes

1. Highly recommend running `panamax-search -U` immmediately following `panamax sync` so that the
   cache file is always up-to-date and ready to process any queries as fast as possible.
   If running `panamax sync` via cron, recommend creating a script like the following and running
   that instead.

   ```bash
   #!/usr/bin/env bash
   set -xeo pipefail
   ~/.cargo/bin/panamax sync ~/panamax
   ~/.cargo/bin/panamax-search -Uv
   ```

