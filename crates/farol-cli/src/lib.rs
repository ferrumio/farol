//! Command-line entry points for farol.
//!
//! Exposed as a library so alternate front-ends (the Python wheel) can call
//! into the same command set with a custom [`PluginHost`].

pub mod serve;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use farol_core::{
    build, plugins, scaffold, BuildOptions, Cache, ChainedHost, Config, NoOpHost, PluginHost,
    DEFAULT_CONFIG_FILENAME,
};
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
pub struct Cli {
    /// Path to the config file.
    #[arg(long, short = 'c', global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Enable verbose logging.
    #[arg(long, short, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error output.
    #[arg(long, short, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
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
pub enum PluginCommand {
    /// Scaffold a new plugin package.
    New {
        /// Plugin name (without the `farol-plugin-` prefix).
        name: String,
    },
    /// List plugins discovered by the current host.
    List,
}

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Remove every cached entry. Next build will be cold.
    Clear,
}

/// Parse `argv` and dispatch, using `host` for plugin hooks.
pub fn run_with_argv<I, T>(argv: I, host: Arc<dyn PluginHost>) -> miette::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(argv);
    run(cli, host)
}

/// Run the CLI using `user_host`.
///
/// `user_host` represents the user's plugin runtime (e.g. Python via
/// `PythonPluginHost`). Builtins are layered on top inside each command
/// after the site config is loaded, so `[plugins] enabled/disabled`
/// filtering can apply uniformly.
pub fn run(cli: Cli, user_host: Arc<dyn PluginHost>) -> miette::Result<()> {
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
            cmd_build(cli.config.as_deref(), timings, no_cache, user_host)
        }
        Some(Commands::Serve { port, host: bind }) => {
            cmd_serve(cli.config.as_deref(), port, bind, user_host)
        }
        Some(Commands::Plugin { cmd }) => match cmd {
            PluginCommand::New { name } => cmd_plugin_new(&name),
            PluginCommand::List => cmd_plugin_list(user_host.as_ref()),
        },
        Some(Commands::Cache { cmd: CacheCommand::Clear }) => {
            cmd_cache_clear(cli.config.as_deref())
        }
    }
}

fn cmd_serve(
    config_path: Option<&Path>,
    port: u16,
    bind: String,
    user_host: Arc<dyn PluginHost>,
) -> miette::Result<()> {
    let (config, project_root) = load_config(config_path)?;
    let composed: Arc<dyn PluginHost> =
        Arc::new(with_builtins_filtered(user_host, &config.plugins));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| miette::miette!("failed to start tokio runtime: {e}"))?;
    runtime.block_on(serve::run(config, project_root, port, bind, composed))
}

fn load_config(config_path: Option<&Path>) -> miette::Result<(Config, PathBuf)> {
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

fn cmd_new(path: &Path) -> miette::Result<()> {
    let created = scaffold::scaffold(path)?;
    println!("Created new farol project at {}", created.display());
    println!();
    println!("  cd {}", path.display());
    println!("  farol serve");
    Ok(())
}

fn cmd_build(
    config_path: Option<&Path>,
    timings: bool,
    no_cache: bool,
    user_host: Arc<dyn PluginHost>,
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

    let host = with_builtins_filtered(user_host, &config.plugins);

    println!("site:      {}", config.site_name);
    println!("config:    {}", config_path.display());
    println!("docs_dir:  {}", config.docs_dir.display());
    println!("site_dir:  {}", config.site_dir.display());
    println!("theme:     {}", config.theme.name);
    if host.name() != "no-op" {
        println!("plugins:   {}", host.name());
    }

    let opts = BuildOptions { timings, no_cache, cache_path: None };
    let report = build::build_with(&config, &project_root, &opts, &host)?;

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

fn cmd_plugin_new(name: &str) -> miette::Result<()> {
    let sanitized = name.trim().trim_start_matches("farol-plugin-");
    let slug: String = sanitized
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        return Err(miette::miette!("plugin name must contain at least one letter or digit"));
    }

    let project_dir = PathBuf::from(format!("farol-plugin-{slug}"));
    if project_dir.exists() {
        return Err(miette::miette!("{} already exists", project_dir.display()));
    }

    let module_name = format!("farol_plugin_{}", slug.replace('-', "_"));
    let package_dir = project_dir.join(&module_name);
    std::fs::create_dir_all(&package_dir).map_err(|e| miette::miette!("mkdir: {e}"))?;

    let tests_dir = project_dir.join("tests");
    std::fs::create_dir_all(&tests_dir).map_err(|e| miette::miette!("mkdir: {e}"))?;

    let pyproject = format!(
        r#"[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "farol-plugin-{slug}"
version = "0.1.0"
description = "A farol plugin."
readme = "README.md"
requires-python = ">=3.9"
license = "Apache-2.0"
dependencies = ["farol"]
keywords = ["farol", "farol-plugin"]

[project.entry-points."farol.plugins"]
{slug} = "{module_name}"

[tool.hatch.build.targets.wheel]
packages = ["{module_name}"]
"#
    );
    std::fs::write(project_dir.join("pyproject.toml"), pyproject)
        .map_err(|e| miette::miette!("write pyproject: {e}"))?;

    let readme = format!(
        r#"# farol-plugin-{slug}

A farol plugin.

## Install

```bash
uv pip install farol-plugin-{slug}
```

The plugin is auto-discovered by any farol project in the same environment.
"#
    );
    std::fs::write(project_dir.join("README.md"), readme)
        .map_err(|e| miette::miette!("write readme: {e}"))?;

    let init = r#""""Sample farol plugin. Replace this hook with your own."""

from farol import hookimpl


@hookimpl
def on_page_markdown(markdown, page, config):
    """Replace :wave: with a friendly waving hand."""
    return markdown.replace(":wave:", "👋")
"#;
    std::fs::write(package_dir.join("__init__.py"), init)
        .map_err(|e| miette::miette!("write __init__: {e}"))?;

    let test = format!(
        r##"from farol.testing import PluginTester
from {module_name} import on_page_markdown


def test_wave_is_replaced():
    result = PluginTester().with_hook(on_page_markdown).build_page("# hi :wave:")
    assert "👋" in result.html
    assert ":wave:" not in result.html
"##
    );
    std::fs::write(tests_dir.join("test_plugin.py"), test)
        .map_err(|e| miette::miette!("write test: {e}"))?;

    println!("Created plugin scaffold at {}", project_dir.display());
    println!();
    println!("  cd {}", project_dir.display());
    println!("  uv venv && uv pip install -e .");
    println!("  uv run pytest");
    Ok(())
}

fn cmd_plugin_list(host: &dyn PluginHost) -> miette::Result<()> {
    let plugins = host.plugins();
    if plugins.is_empty() {
        println!("no plugins loaded (host: {})", host.name());
    } else {
        println!("{} plugin(s) loaded via {}:", plugins.len(), host.name());
        for p in plugins {
            println!("  - {p}");
        }
    }
    Ok(())
}

fn cmd_cache_clear(config_path: Option<&Path>) -> miette::Result<()> {
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

/// Entry point for the standalone (non-Python) binary.
///
/// Passes a bare [`NoOpHost`] as the "user host". Builtins are composed on
/// top in each command after the site's config has been loaded, so that
/// `[plugins] enabled/disabled` can be honored.
pub fn main_entry() -> miette::Result<()> {
    let host: Arc<dyn PluginHost> = Arc::new(NoOpHost);
    run_with_argv(std::env::args(), host)
}

/// Wrap a user host with all builtin plugins. User hooks run before builtins
/// so their transformations feed into the builtin pipeline.
pub fn with_builtins(user: Arc<dyn PluginHost>) -> ChainedHost {
    with_builtins_filtered(user, &farol_core::config::PluginsConfig::default())
}

/// Like [`with_builtins`] but honors `[plugins] enabled/disabled` from the
/// site's config, filtering the builtin set before composing.
pub fn with_builtins_filtered(
    user: Arc<dyn PluginHost>,
    plugins_cfg: &farol_core::config::PluginsConfig,
) -> ChainedHost {
    let mut hosts: Vec<Arc<dyn PluginHost>> = vec![user];
    for plugin in plugins::core::all() {
        if plugins_cfg.is_plugin_enabled(plugin.name()) {
            hosts.push(Arc::from(plugin));
        }
    }
    ChainedHost::new(hosts)
}
