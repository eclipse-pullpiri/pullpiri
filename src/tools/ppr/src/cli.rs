use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct PullpiriCli {
    #[command(subcommand)]
    pub command: Commands,
    #[arg(short, long, default_value_t = false)]
    pub logo: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Apply(YamlInfo),
    Delete(YamlInfo),
    Status,
}

#[derive(Args, Debug)]
pub struct YamlInfo {
    name: String,
}

pub fn parse() -> PullpiriCli {
    PullpiriCli::parse()
}
