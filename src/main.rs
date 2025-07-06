mod ignore;
mod index;
mod kvlm;
mod object;
mod refs;
mod repository;

use clap::{Parser, Subcommand};
use log::{error, info};
use object::GitrsObject::{CommitObject, TreeObject};
use object::commit::Commit;
use object::tag::{Tag, TagType};
use object::tree::Leaf;
use object::{GitrsObject, ObjectFindOptions, ObjectType};
use refs::Ref;
use repository::Repository;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Gitrs CLI commands
#[derive(Subcommand, Debug)]
enum Command {
    /// Initialize a gitrs repository
    ///
    /// Defaults to current directory if no path provided
    Init {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Read a file, hash its contents, and store it in the repository
    HashObject {
        #[arg(value_parser)]
        object_type: ObjectType,
        path: String,
    },
    /// Print raw (uncompressed, no header) contents of an object to stdout
    CatFile { object: String },
    /// Log commits starting from a specified commit (default HEAD)
    Log {
        #[arg(default_value = "HEAD")]
        commit: String,
    },
    /// List tree contents, optionally recursively
    LsTree {
        #[arg(short = 'r', long = "recursive")]
        recursive: bool,
        tree: String,
    },
    /// Checkout a commit into a specified directory
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
    /// Resolve references to object hashes
    RevParse {
        #[arg(value_parser)]
        object_type: ObjectType,
        name: String,
    },
    /// Check ignore rules against specified paths
    CheckIgnore {
        #[arg(required = true)]
        paths: Vec<String>,
    },
}

/// Main CLI struct for gitrs
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Gitrs {
    #[command(subcommand)]
    cmd: Command,
}

fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let gitrs = Gitrs::parse();

    match gitrs.cmd {
        Command::Init { path } => {
            // Initialize a new repository at the given path
            match Repository::init(Path::new(&path)) {
                Ok(_) => info!("Successfully initialized git repository"),
                Err(e) => error!("Error initializing repository: {}", e),
            }
        }

        Command::HashObject { object_type, path } => {
            // Read file, hash, and write as git object
            let file = File::open(&path).expect("Could not open file");
            let mut data = Vec::new();
            BufReader::new(file)
                .read_to_end(&mut data)
                .expect("Could not read file");

            let repository = Repository::find_repository();
            let hash = GitrsObject::deserialize_and_write(&repository, &data, object_type);
            info!("{}", hash);
        }

        Command::CatFile { object } => {
            // TODO: accept type argument for more precise lookup
            let repository = Repository::find_repository();

            let hash = GitrsObject::find(&repository, &object, None)
                .unwrap_or_else(|_| panic!("Couldn't find object named '{}'", object));

            let mut obj = GitrsObject::read(&repository, &hash)
                .unwrap_or_else(|_| panic!("Couldn't read object with hash '{}'", hash));

            info!("Object contents:");
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
            .unwrap_or_else(|_| panic!("Couldn't find commit named '{}'", commit));

            if let Ok(CommitObject(commit_obj)) = GitrsObject::read(&repository, &hash) {
                info!("[{}] {}", Commit::short(&hash), commit_obj.message());
            } else {
                panic!("Expected commit object for hash {}", hash);
            }
        }

        Command::LsTree { recursive, tree } => {
            let repository = Repository::find_repository();

            if let Ok(TreeObject(tree_obj)) = GitrsObject::read(&repository, &tree) {
                // TODO: fix formatting and implement recursive listing
                for leaf in &tree_obj.records {
                    let obj_type = Leaf::get_type_from_mode(&leaf.file_mode);
                    info!("Found {} object", obj_type);
                }
            } else {
                panic!("Expected a tree object for {}", tree);
            }
        }

        Command::Checkout {
            commit,
            path: path_str,
        } => {
            let path = Path::new(&path_str);

            // TODO: create directory if it doesn't exist
            if !repository::is_empty_dir(path) {
                panic!("Expected an empty directory at {}", path_str);
            }

            let repository = Repository::find_repository();

            let commit_obj = match GitrsObject::read(&repository, &commit) {
                Ok(CommitObject(obj)) => obj,
                _ => panic!("Expected a commit object for {}", commit),
            };

            let tree_obj = match GitrsObject::read(&repository, commit_obj.get_tree_hash()) {
                Ok(TreeObject(obj)) => obj,
                _ => panic!("Couldn't find tree for commit {}", commit),
            };

            tree_obj
                .checkout(&repository, path)
                .expect("Error occurred during checkout");
        }

        Command::ShowRef => {
            let repository = Repository::find_repository();

            let refs = Ref::list_at(
                &repository,
                &repository
                    .get_path_to_dir(&["refs"])
                    .expect("Expected refs dir"),
            )
            .expect("Couldn't resolve refs");

            // TODO: implement pretty print for refs
            for (ref_key, ref_val) in refs.iter() {
                info!("ref: {} {}", ref_key, ref_val);
            }
        }

        Command::Tag {
            annotated,
            name,
            object,
        } => {
            let repository = Repository::find_repository();

            match name {
                Some(tag_name) => {
                    let tag_type = if annotated {
                        TagType::Object
                    } else {
                        TagType::Lightweight
                    };

                    if let Some(obj_ref) = object {
                        let hash = GitrsObject::find(&repository, &obj_ref, None)
                            .unwrap_or_else(|_| panic!("Couldn't find object named '{}'", obj_ref));

                        Tag::create(&repository, &tag_name, &hash, tag_type)
                            .expect("Couldn't create tag");
                    } else {
                        panic!("Must provide object reference when creating a tag");
                    }
                }
                None => {
                    // TODO: extract common pretty print method for refs and tags
                    let refs = Ref::list_at(
                        &repository,
                        &repository
                            .get_path_to_dir(&["refs"])
                            .expect("Expected refs dir"),
                    )
                    .expect("Couldn't resolve refs");

                    for (ref_key, ref_val) in refs.iter() {
                        info!("ref: {} {}", ref_key, ref_val);
                    }
                }
            }
        }

        Command::RevParse { object_type, name } => {
            let repository = Repository::find_repository();

            let hash = GitrsObject::find(
                &repository,
                &name,
                Some(ObjectFindOptions {
                    object_type,
                    should_follow: true,
                }),
            )
            .unwrap_or_else(|_| panic!("Couldn't find object named '{}'", name));

            info!("{}", hash);
        }

        Command::CheckIgnore { paths: _ } => {
            // TODO: Implement ignore checking logic
            let _repository = Repository::find_repository();
        }
    };
}
