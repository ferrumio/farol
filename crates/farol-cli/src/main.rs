use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "farol", version = farol_core::VERSION, about = "docs, lit.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the site.
    Build,
    /// Serve the site with live reload.
    Serve,
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Build) => println!("farol build: not implemented yet"),
        Some(Commands::Serve) => println!("farol serve: not implemented yet"),
        None => println!("farol {} - run `farol --help`", farol_core::VERSION),
    }
    Ok(())
}
