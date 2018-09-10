
#[macro_use] extern crate clap;
extern crate encoding_rs_io;
extern crate git2;
extern crate regex;
extern crate tokei;

mod changes;

use std::env;
use std::error::Error;
use std::collections::HashMap;
use std::sync::Mutex;
use std::process;

use git2::{Repository, DiffDelta, DiffOptions, Oid};

use changes::Changes;

fn main() -> Result<(), Box<Error>> {
    let matches = clap_app!(hermes =>
        (version: crate_version!())
        (author: "Aaron P. <theaaronepower@gmail.com> + Contributors")
        (about: crate_description!())
        (@arg command: -c --command
            +takes_value
            "Command to run if there were code changes.")
        (@arg input:
            conflicts_with[languages] ...
            "The git repos to be checked. Defaults to the current directory.")
        (@arg quiet: -q --quiet
            conflicts_with[verbose]
            "Do not output to stdout.")
        (@arg filter: -f --filter
            +takes_value
            "Filters by language, seperated by a comma. i.e. -t=Rust,C")
        (@arg verbose: -v --verbose ...
            "Set log output level:
            1: to show unknown file extensions,
            2: reserved for future debugging,
            3: enable file level trace. Not recommended on multiple files")
    ).get_matches();

    let paths: Vec<&str> = matches.values_of("input")
                                  .map(|v| v.collect())
                                  .unwrap_or(vec!["."]);

    for path in paths {
        env::set_current_dir(path)?;
        let repo = Repository::open(".")?;

        let head = repo.head()?.peel_to_commit()?;
        let previous = head.parents().next().unwrap();
        let mut options = DiffOptions::new();
        options.context_lines(0);

        let diff = repo.diff_tree_to_tree(
            Some(&previous.tree()?),
            Some(&head.tree()?),
            Some(&mut options),
        )?;

        let mut diff_map = Mutex::new(HashMap::new());

        diff.foreach(
            &mut |delta, _| {
                diff_map.lock().unwrap().insert(delta_to_oids(&delta), Changes::new(&delta));
                true
            },
            None,
            None,
            Some(&mut |delta, _, line| {
                let mut map = diff_map.lock().unwrap();
                let changes = map.get_mut(&delta_to_oids(&delta)).unwrap();

                if let Some(number) = line.old_lineno() {
                    changes.removed.lines.push(number);
                } else if let Some(number) = line.new_lineno() {
                    changes.added.lines.push(number);
                }

                true
            }),
        )?;

        for (_, changes) in diff_map.lock().unwrap().iter() {
            if changes.added.has_code_changes()? {
                println!("Code has changed.");
                process::exit(0);
            }
        }

        repo.checkout_tree(previous.as_object(), None)?;

        for (_, changes) in diff_map.lock().unwrap().iter() {
            if changes.removed.has_code_changes()? {
                println!("Code has changed.");
                process::exit(0);
            }
        }
    }

    println!("No code has changed.");
    process::exit(1)
}

fn delta_to_oids(delta: &DiffDelta) -> (Oid, Oid) {
    (delta.old_file().id(), delta.new_file().id())
}

#[cfg(test)]
mod tests {
    extern crate assert_cmd;
    extern crate tempfile;

    use std::{env, fs, path::PathBuf, process::Command};

    use self::assert_cmd::prelude::*;
    use git2::Repository;

    macro_rules! git {
        ($($tree:expr)*) => {{
            Command::new("git")
                .args(&[$($tree,)*])
                .status()
                .unwrap()
        }}
    }

    fn create_and_init_tempdir() -> (PathBuf, tempfile::TempDir) {
        let current = env::current_dir().unwrap();
        let temp = tempfile::tempdir().expect("Couldn't create tempdir.");
        env::set_current_dir(temp.path()).expect("Couldn't go to tempdir.");
        Repository::init(temp.path()).unwrap();

        (current, temp)
    }

    #[test]
    fn code_line_changed() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.rs"), b"Hello\n")
            .expect("Couldn't write to lib.rs");

        git!("add" ".");
        git!("commit" "-m" "Initial Commit");

        fs::write(temp.path().join("lib.rs"), b"Hello\nWorld")
            .expect("Couldn't write to lib.rs");

        git!("add" ".");
        git!("commit" "-m" "Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("Code has changed.\n")
            .success();
    }

    #[test]
    fn comment_line_changed() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.rs"), b"// Hello\n")
            .expect("Couldn't write to lib.rs");

        git!("add" ".");
        git!("commit" "-m" "Initial Commit");

        fs::write(temp.path().join("lib.rs"), b"// Hello\n// World")
            .expect("Couldn't write to lib.rs");

        git!("add" ".");
        git!("commit" "-m" "Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("No code has changed.\n")
            .failure();
    }
}
