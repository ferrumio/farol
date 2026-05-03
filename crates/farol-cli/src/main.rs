mod serve;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use farol_core::{build, scaffold, BuildOptions, Cache, Config, DEFAULT_CONFIG_FILENAME};
use tracing_subscriber::EnvFilter;

const BANNER: &str = r#"
   ╔═╗
   ║ ║  ◦ ◦ ◦ ◦
   ║ ║
  ╔╩═╩╗
  ║   ║
 ╔╝   ╚╗
 ╚═════╝
"#;

#[derive(Parser)]
#[command(
    name = "farol",
    version = farol_core::VERSION,
    about = "docs, lit.",
    long_about = "farol - fast, plugin-first documentation generator. Forged in Rust. Lit for life.",
    propagate_version = true
)]
struct Cli {
    /// Path to the config file.
    #[arg(long, short = 'c', global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Enable verbose logging.
    #[arg(long, short, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress non-error output.
    #[arg(long, short, global = true, conflicts_with = "verbose")]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new farol project.
    New {
        /// Directory to create.
        path: PathBuf,
    },
    /// Build the site.
    Build {
        /// Print per-node timing and cache hit rate.
        #[arg(long)]
        timings: bool,
        /// Skip the build cache entirely.
        #[arg(long)]
        no_cache: bool,
    },
    /// Serve the site with live reload.
    Serve {
        /// Port to bind.
        #[arg(long, short = 'p', default_value_t = 8000)]
        port: u16,
        /// Host to bind. Use 0.0.0.0 to expose on the LAN.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Work with plugins.
    Plugin {
        #[command(subcommand)]
        cmd: PluginCommand,
    },
    /// Inspect or clear the build cache.
    Cache {
        #[command(subcommand)]
        cmd: CacheCommand,
    },
}

#[derive(Subcommand)]
enum PluginCommand {
    /// Scaffold a new plugin package (coming soon).
    New {
        /// Plugin name (without the `farol-plugin-` prefix).
        name: String,
    },
}

#[derive(Subcommand)]
enum CacheCommand {
    /// Remove every cached entry. Next build will be cold.
    Clear,
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose, cli.quiet);

    match cli.command {
        None => {
            println!("{BANNER}");
            println!("  farol {}", farol_core::VERSION);
            println!("  run `farol --help` for usage");
            Ok(())
        }
        Some(Commands::New { path }) => cmd_new(&path),
        Some(Commands::Build { timings, no_cache }) => {
            cmd_build(cli.config.as_deref(), timings, no_cache)
        }
        Some(Commands::Serve { port, host }) => cmd_serve(cli.config.as_deref(), port, host),
        Some(Commands::Plugin { cmd: PluginCommand::New { name } }) => {
            println!("farol plugin new {name}: not implemented yet");
            Ok(())
        }
        Some(Commands::Cache { cmd: CacheCommand::Clear }) => {
            cmd_cache_clear(cli.config.as_deref())
        }
    }
}

fn cmd_serve(config_path: Option<&std::path::Path>, port: u16, host: String) -> miette::Result<()> {
    let (config, project_root) = load_config(config_path)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| miette::miette!("failed to start tokio runtime: {e}"))?;
    runtime.block_on(serve::run(config, project_root, port, host))
}

fn load_config(config_path: Option<&std::path::Path>) -> miette::Result<(Config, PathBuf)> {
    let config_path = config_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILENAME));
    let project_root =
        config_path.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let config = if config_path.exists() {
        Config::load(&config_path)?
    } else {
        tracing::warn!(path = %config_path.display(), "config not found, using defaults");
        Config::default()
    };
    Ok((config, project_root))
}

fn cmd_new(path: &std::path::Path) -> miette::Result<()> {
    let created = scaffold::scaffold(path)?;
    println!("Created new farol project at {}", created.display());
    println!();
    println!("  cd {}", path.display());
    println!("  farol serve");
    Ok(())
}

fn cmd_build(
    config_path: Option<&std::path::Path>,
    timings: bool,
    no_cache: bool,
) -> miette::Result<()> {
    let config_path = config_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILENAME));

    let project_root =
        config_path.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));

    let config = if config_path.exists() {
        Config::load(&config_path)?
    } else {
        tracing::warn!(path = %config_path.display(), "config not found, using defaults");
        Config::default()
    };

    println!("site:      {}", config.site_name);
    println!("config:    {}", config_path.display());
    println!("docs_dir:  {}", config.docs_dir.display());
    println!("site_dir:  {}", config.site_dir.display());
    println!("theme:     {}", config.theme.name);

    let opts = BuildOptions { timings, no_cache, cache_path: None };
    let report = build::build_with(&config, &project_root, &opts)?;

    println!();
    println!("built {} pages, {} assets", report.pages, report.assets);
    if !report.broken_links.is_empty() {
        println!("warning: {} broken link(s):", report.broken_links.len());
        for b in &report.broken_links {
            println!("  {} -> {} ({})", b.page.display(), b.href, b.reason);
        }
    }

    if let Some(graph) = &report.graph {
        println!();
        println!(
            "graph: {} nodes in {:.0?} ({} hit, {} miss, {:.0}% hit rate)",
            graph.total_nodes(),
            graph.total_elapsed,
            graph.cache_hits,
            graph.cache_misses,
            graph.hit_rate() * 100.0,
        );
        for t in &graph.timings {
            let tag = match t.outcome {
                farol_core::NodeOutcome::Hit => "HIT ",
                farol_core::NodeOutcome::Miss => "MISS",
            };
            println!("  {tag}  {:>8.2?}  {}", t.elapsed, t.id);
        }
    }

    Ok(())
}

fn cmd_cache_clear(config_path: Option<&std::path::Path>) -> miette::Result<()> {
    let config_path = config_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILENAME));
    let project_root =
        config_path.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let cache_path = project_root.join(".farol").join("cache.redb");

    if !cache_path.exists() {
        println!("no cache at {} (nothing to clear)", cache_path.display());
        return Ok(());
    }

    let cache = Cache::open(&cache_path)?;
    cache.clear()?;
    println!("cleared cache at {}", cache_path.display());
    Ok(())
}

fn init_tracing(verbose: u8, quiet: bool) {
    let level = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).with_target(false).try_init();
}
