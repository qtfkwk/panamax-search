use {
    anyhow::{anyhow, Result},
    clap::{ArgAction::Count, Parser},
    expanduser::expanduser,
    log::LevelFilter::*,
    panamax_search_lib::Index,
};

#[derive(Parser)]
#[command(about, version, max_term_width = 80)]
struct Cli {
    /// Force update the cache file and exit
    #[arg(short = 'U', conflicts_with_all = ["search", "include_yanked", "case_sensitive"])]
    update: bool,

    /// Mirror directory
    #[arg(short, value_name = "PATH", default_value = "~/panamax")]
    mirror: String,

    /// Verbose (default=warn; -v=info; -vv=debug; -vvv=trace)
    #[arg(short, action = Count)]
    verbose: u8,

    /// Include yanked
    #[arg(short = 'y')]
    include_yanked: bool,

    /// Case sensitive
    #[arg(short = 's')]
    case_sensitive: bool,

    /// Search queries
    #[arg(value_name = "QUERY")]
    search: Vec<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    env_logger::builder()
        .filter_level(match cli.verbose {
            0 => Warn,  // Default level
            1 => Info,  // -v
            2 => Debug, // -vv
            _ => Trace, // -vvv
        })
        .init();

    let mirror = expanduser(&cli.mirror).unwrap();

    if cli.update {
        // Force update and exit
        Index::load_from_mirror_directory(&mirror)?;
        Ok(())
    } else if cli.search.is_empty() {
        Err(anyhow!("No search query"))
    } else {
        let index = Index::load(&mirror)?;
        println!(
            "{}",
            index
                .search(&cli.search, !cli.case_sensitive)
                .to_string(cli.include_yanked, true),
        );
        Ok(())
    }
}
