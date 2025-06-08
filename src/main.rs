mod object;
mod repository;

use clap::{Parser, Subcommand};
use object::GitrsObject;
use repository::Repository;
use std::{env, path::Path};

#[derive(Subcommand, Debug)]
enum Command {
    /// Initialize a gitrs repository
    ///
    /// The path defaults to the directory the gitrs init command is invoked in
    Init {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Prints the raw contents of an object (uncompressed and without the git header) to the
    /// stdout
    CatFile {
        #[arg(value_parser)]
        object_type: object::ObjectType,
        hash: String,
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
        Command::Init { path } => {
            match Repository::init(Path::new(&path)) {
                Ok(_) => println!("Successfully initialized git repository"),
                Err(e) => println!("An error occurred initializing gitrs repository: {}", e),
            };
        }
        Command::CatFile { object_type, hash } => {
            let repository =
                // TODO: encode test directory in a config file or something
                Repository::find_repository(&env::current_dir().unwrap().join("test").as_path())
                    .unwrap();
            let obj = GitrsObject::object_read(&repository, hash);
            println!("{}", hex::encode(obj.serialize()));
        }
    };
}
