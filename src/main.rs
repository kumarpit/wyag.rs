mod repository;

use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Subcommand, Debug)]
enum Command {
    /// Initialize a gitrs repository
    Init { path: String },
}

/// A light-weight git clone written in Rust
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Gitrs {
    #[command(subcommand)]
    cmd: Command,
}

fn main() {
    let gitrs = Gitrs::parse();

    match gitrs.cmd {
        Command::Init { path } => repository::Repository::new(Path::new(&path)),
    };
}
