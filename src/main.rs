use crate::app::App;
use clap::Parser;
use cli::{Cli, Commands};

mod action;
mod app;
mod cli;
mod data_pipeline;
mod component_manager;
mod components;
mod config;
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
            Commands::Add(_) => {}
            Commands::Remove(_) => {}
            Commands::List => {
                let regexes = config::get_regexes().unwrap();
                for (name, regex) in regexes {
                    println!("{:<10}: {}", name, regex);
                }
            }
        }
    } else {
        let mut app = App::new(args)?;
        app.run().await?;
    }
    Ok(())
}
