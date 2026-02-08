pub mod detector;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "openrunner")]
#[command(about = "OpenRunner - Desktop process manager for local development")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new group and auto-detect projects
    #[command(name = "new")]
    New {
        /// Directory to scan for projects (default: current directory)
        #[arg(default_value = ".")]
        directory: PathBuf,

        /// Group name (default: directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Dry run - show what would be created without making changes
        #[arg(long)]
        dry_run: bool,
    },
}

pub fn parse_cli() -> Cli {
    Cli::parse()
}

/// Run CLI mode - returns true if CLI was invoked, false to run GUI
pub fn run_cli() -> bool {
    let cli = parse_cli();

    match cli.command {
        Some(Commands::New {
            directory,
            name,
            dry_run,
        }) => {
            if let Err(e) = detector::execute_new(directory, name, dry_run) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            true
        }
        None => false, // No subcommand, run GUI
    }
}
