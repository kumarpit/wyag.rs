mod object;
mod repository;

use clap::{Parser, Subcommand};
use repository::Repository;
use std::path::Path;

#[derive(Subcommand, Debug)]
enum Command {
    /// Initialize a gitrs repository
    ///
    /// The path defaults to the directory the gitrs init command is invoked in
    Init {
        #[arg(default_value = ".")]
        path: String,
    },
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
        Command::Init { path } => Repository::init(Path::new(&path)),
    };
}
