use std::{self};

use clap::{Parser, Subcommand};

mod dialogue;
mod repos;
mod cd;

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    // Make this command default
    #[clap(flatten)]
    find_project: FindProjectArgs,
}

#[derive(Subcommand, Debug)]
enum Commands {
    FindProject(FindProjectArgs),
    OnChangedDirectory { path: String },
    ExpandCd { path: String },
    Init {},
}

#[derive(Debug, clap::Args)]
struct FindProjectArgs {
    #[arg(default_value = ".")]
    path: String,

    #[arg(short, long)]
    new_tab: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    return match cli
        .command
        .unwrap_or(Commands::FindProject(cli.find_project))
    {
        Commands::FindProject(FindProjectArgs { path, new_tab }) => {
            return repos::find_project(path, new_tab);
        }
        Commands::OnChangedDirectory { path } => {
            cd::on_changed_directory(&path)?;
            return Ok(());
        },
        Commands::ExpandCd { path } => {
            return cd::expand(&path);
        },
        Commands::Init {} => {
            let content = include_str!("init.ps1");
            print!("{:}", content);
            return Ok(());
        }

        _ => Ok(()),
    };
}

