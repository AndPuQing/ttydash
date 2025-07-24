use crate::app::App;
use clap::Parser;
use cli::{Cli, Commands};

mod action;
mod app;
mod cli;
mod component_manager;
mod components;
mod config;
mod data_pipeline;
mod errors;
mod logging;
mod tui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    if let Some(cmd) = &args.cmd {
        match cmd {
            Commands::Add(args) => {
                if let Err(e) = config::add_regex(&args.name, &args.regex) {
                    eprintln!("Error adding regex: {e}");
                }
            }
            Commands::Remove(args) => {
                if let Err(e) = config::remove_regex(&args.name) {
                    eprintln!("Error removing regex: {e}");
                }
            }
            Commands::List => {
                let regexes = config::get_regexes().unwrap();
                for (name, regex) in regexes {
                    println!("{name:<10}: {regex}");
                }
            }
        }
    } else {
        let mut app = App::new(args)?;
        app.run().await?;
    }
    Ok(())
}
