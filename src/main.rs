mod kvlm;
mod object;
mod repository;

use clap::{Parser, Subcommand};
use object::GitrsObject::{CommitObject, TreeObject};
use object::commit::Commit;
use object::tree::Leaf;
use object::{GitrsObject, ObjectType};
use repository::Repository;
use std::fs::File;
use std::io::{BufReader, Read};
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
    /// Reads the given file (path relative to the repository), computes its hash,
    /// and stores it in the repository
    HashObject {
        #[arg(value_parser)] // NOTE: Target type must implement FromStr
        object_type: ObjectType,
        path: String,
    },
    /// Prints the raw contents of an object (uncompressed and without the git header) to the
    /// stdout
    CatFile { hash: String },
    /// Logs commits on the current branch
    Log {
        #[arg(default_value = "HEAD")]
        commit: String,
    },
    LsTree {
        #[arg(short = 'r', long = "recursive")]
        recursive: bool,
        tree: String,
    },
    /// Checkout a commit inside of a directory
    Checkout { commit: String, path: String },
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

            let repository = Repository::find_repository();
            let hash = GitrsObject::write(&repository, data.as_slice(), object_type);
            println!("{}", hash);
        }
        Command::CatFile { hash } => {
            let repository = Repository::find_repository();
            let mut obj = GitrsObject::read(&repository, &hash).unwrap();
            print!("Object contents");
            GitrsObject::dump(&obj.serialize());
        }
        Command::Log { commit } => {
            // TODO: list all commits, also default the commit to HEAD rather than using the actual
            // hash
            // This can be achieved using the `object_find` method

            let repository = Repository::find_repository();
            if let Ok(CommitObject(commit_obj)) = GitrsObject::read(&repository, &commit) {
                println!("[{}] {}", Commit::short(&commit), commit_obj.message());
            } else {
                panic!("Expected commit");
            }
        }
        Command::LsTree { recursive, tree } => {
            let repository = Repository::find_repository();
            if let Ok(TreeObject(tree_obj)) = GitrsObject::read(&repository, &tree) {
                // TODO : fix formatting of the lst tree message
                // and implement recursive handling
                tree_obj.records.iter().for_each(|leaf| {
                    let obj_type = Leaf::get_type_from_mode(&leaf.file_mode);
                    println!("Found {} object", &obj_type.to_string());
                })
            } else {
                panic!("Expected a tree");
            }
        }
        Command::Checkout {
            commit,
            path: path_str,
        } => {
            let path = Path::new(&path_str);
            // TODO: could also make a dir if not exists
            if !repository::is_empty_dir(path) {
                panic!("Expected an empty dir at {}", path_str);
            }

            let repository = Repository::find_repository();
            let Ok(CommitObject(commit_obj)) = GitrsObject::read(&repository, &commit) else {
                panic!("Expected a commit object");
            };

            let Ok(TreeObject(tree_obj)) =
                GitrsObject::read(&repository, commit_obj.get_tree_hash())
            else {
                panic!("Couldn't find tree for {}", commit);
            };

            tree_obj
                .checkout(&repository, path)
                .expect("An error occurred during checkout");
        }
    };
}
