mod object;
mod repository;

use clap::{Parser, Subcommand};
use object::GitrsObject;
use repository::Repository;
use std::fs::File;
use std::io::{BufReader, Read};
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
    /// Reads the given file (path relative to the repository), computes its hash,
    /// and stores it in the repository
    HashObject {
        #[arg(value_parser)]
        object_type: object::ObjectType,
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
        Command::HashObject { object_type, path } => {
            let file = File::open(path).expect("Could not open file");
            let mut data = Vec::new();
            let _size = BufReader::new(file)
                .read_to_end(&mut data)
                .expect("Could not read file");
            let repository =
                Repository::find_repository(&env::current_dir().unwrap().as_path()).unwrap();
            GitrsObject::write(&repository, data.as_slice(), object_type);
        }
        Command::CatFile { object_type, hash } => {
            let repository =
                Repository::find_repository(&env::current_dir().unwrap().as_path()).unwrap();
            let obj = GitrsObject::object_read(&repository, hash);

            println!("Object contents");
            GitrsObject::dump(&obj.serialize().to_vec());
        }
    };
}
