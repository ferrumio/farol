use std::path::PathBuf;

use clap::{Parser, Subcommand};
use farol_core::{build, scaffold, Config, DEFAULT_CONFIG_FILENAME};
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
    Build,
    /// Serve the site with live reload.
    Serve,
    /// Work with plugins.
    Plugin {
        #[command(subcommand)]
        cmd: PluginCommand,
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
        Some(Commands::Build) => cmd_build(cli.config.as_deref()),
        Some(Commands::Serve) => {
            println!("farol serve: not implemented yet");
            Ok(())
        }
        Some(Commands::Plugin { cmd: PluginCommand::New { name } }) => {
            println!("farol plugin new {name}: not implemented yet");
            Ok(())
        }
    }
}

fn cmd_new(path: &std::path::Path) -> miette::Result<()> {
    let created = scaffold::scaffold(path)?;
    println!("Created new farol project at {}", created.display());
    println!();
    println!("  cd {}", path.display());
    println!("  farol serve");
    Ok(())
}

fn cmd_build(config_path: Option<&std::path::Path>) -> miette::Result<()> {
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

    let report = build::build(&config, &project_root)?;

    println!();
    println!("built {} pages, {} assets", report.pages, report.assets);
    if !report.broken_links.is_empty() {
        println!("warning: {} broken link(s):", report.broken_links.len());
        for b in &report.broken_links {
            println!("  {} -> {} ({})", b.page.display(), b.href, b.reason);
        }
    }

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
