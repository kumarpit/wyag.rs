mod kvlm;
mod object;
mod refs;
mod repository;

use clap::{Parser, Subcommand};
use object::GitrsObject::{CommitObject, TreeObject};
use object::ObjectFindOptions;
use object::commit::Commit;
use object::tag::{Tag, TagType};
use object::tree::Leaf;
use object::{GitrsObject, ObjectType};
use refs::Ref;
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
    CatFile { object: String },
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
    /// List references
    ShowRef,
    /// Create or list tags
    Tag {
        #[arg(short = 'a', long = "annotated")]
        annotated: bool,
        name: Option<String>,
        object: Option<String>,
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

            let repository = Repository::find_repository();
            let hash =
                GitrsObject::deserialize_and_write(&repository, data.as_slice(), object_type);
            println!("{}", hash);
        }
        Command::CatFile { object } => {
            let repository = Repository::find_repository();

            let hash = GitrsObject::find(&repository, &object, None)
                .expect(&format!("Couldn't find object with name: {}", object));

            let mut obj = GitrsObject::read(&repository, &hash)
                .expect(&format!("Couldn't read object with hash: {}", hash));

            print!("Object contents");
            GitrsObject::dump(&obj.serialize());
        }
        Command::Log { commit } => {
            let repository = Repository::find_repository();

            let hash = GitrsObject::find(
                &repository,
                &commit,
                Some(ObjectFindOptions {
                    object_type: ObjectType::Commit,
                    should_follow: false,
                }),
            )
            .expect(&format!("Couldn't find commit with name: {}", commit));

            if let Ok(CommitObject(commit_obj)) = GitrsObject::read(&repository, &hash) {
                println!("[{}] {}", Commit::short(&hash), commit_obj.message());
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
        Command::ShowRef => {
            let repository = Repository::find_repository();
            let refs = Ref::list_at(
                &repository,
                &repository
                    .get_path_to_dir(&["refs"])
                    .expect("Expected refs dir to exist"),
            )
            .expect("Couldn't resolve all refs");

            // TODO: Make a pretty print function for refs
            for (ref_key, ref_val) in refs.iter() {
                println!("ref: {:?} {}", ref_key, ref_val);
            }
        }
        Command::Tag {
            annotated,
            name: name_opt,
            object: object_opt,
        } => {
            let repository = Repository::find_repository();
            // List or create is decided by whether the NAME arg is provided
            match name_opt {
                Some(name) => {
                    let tag_type = if annotated {
                        TagType::Object
                    } else {
                        TagType::Lightweight
                    };

                    if let Some(object) = object_opt {
                        let hash = GitrsObject::find(&repository, &object, None)
                            .expect(&format!("Couldn't find object with name: {}", object));

                        Tag::create(&repository, &name, &hash, tag_type)
                            .expect("Couldn't create tag");
                    } else {
                        panic!("Must provide object reference if creating a tag");
                    }
                }
                None => {
                    // TODO: extract into a common method
                    let refs = Ref::list_at(
                        &repository,
                        &repository
                            .get_path_to_dir(&["refs"])
                            .expect("Expected refs dir to exist"),
                    )
                    .expect("Couldn't resolve all refs");

                    // TODO: Make a pretty print function for refs
                    for (ref_key, ref_val) in refs.iter() {
                        println!("ref: {} {}", ref_key, ref_val);
                    }
                }
            }
        }
    };
}
