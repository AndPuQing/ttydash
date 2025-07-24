use clap::ArgAction;
use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

use crate::config::get_config_dir;
use crate::config::get_data_dir;

#[derive(Debug, ValueEnum, Clone, PartialEq, Eq, Default)]
pub enum Layout {
    Horizontal,
    Vertical,
    #[default]
    Auto,
}

#[derive(Parser, Debug, Clone)]
#[command(author, version = version(), about)]
pub struct Cli {
    /// Tick rate, i.e. number of ticks per second
    #[arg(long, value_name = "FLOAT", default_value_t = 4.0)]
    pub tick_rate: f64,

    /// Frame rate, i.e. number of frames per second
    #[arg(short, long, value_name = "FLOAT", default_value_t = 60.0)]
    pub frame_rate: f64,

    /// Chart title, will be shown at the top of the chart
    #[arg(short, long, value_name = "STRING")]
    pub titles: Option<Vec<String>>,

    /// Unit to be used in the chart (e.g. "ms", "MB")
    #[arg(short, long)]
    pub units: Option<Vec<String>>,

    /// Index vector to be used in the chart
    #[arg(short, long, value_name = "INT")]
    pub indices: Option<Vec<usize>>,

    /// Group together to show multiple charts in the same window
    #[clap(
        short,
        long,
        default_missing_value("true"),
        default_value("false"),
        num_args(0..=1),
        require_equals(true),
        action = ArgAction::Set,
    )]
    pub group: Option<bool>,

    /// Update frequency, i.e. number of milliseconds between updates
    #[arg(long, value_name = "INT", default_value_t = 1000)]
    pub update_frequency: u64,

    /// Layout of the chart
    #[clap(short, long, value_name = "STRING", default_value("auto"))]
    pub layout: Option<Layout>,

    #[command(subcommand)]
    pub cmd: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Add a new regex to the list of regexes
    Add(AddArgs),
    /// Remove a regex from the list of regexes
    Remove(RemoveArgs),
    /// List all regexes
    List,
}
#[derive(Args, Debug, Clone)]
pub struct AddArgs {
    /// Name of the regex
    #[arg(short, long)]
    pub name: String,
    /// The regex to add
    #[arg(short, long)]
    pub regex: String,
}

#[derive(Args, Debug, Clone)]
pub struct RemoveArgs {
    /// The name of the regex to remove
    #[arg(short, long)]
    name: String,
}

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_DESCRIBE"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

pub fn version() -> String {
    let author = clap::crate_authors!();

    let config_dir_path = get_config_dir().display().to_string();
    let data_dir_path = get_data_dir().display().to_string();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}"
    )
}
