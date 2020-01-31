
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
use tokei::LanguageType;

use changes::Changes;

fn main() -> Result<(), Box<Error>> {
    let matches = clap_app!(hera =>
        (version: crate_version!())
        (author: "Erin P. <xampprocky@gmail.com> + Contributors")
        (about: crate_description!())
        (@arg input:
            conflicts_with[languages] ...
            "The git repositories to be checked. Defaults to the \
            current directory.")
        (@arg quiet: -q --quiet
            conflicts_with[verbose]
            "Do not output to stdout.")
        (@arg filter: -f --filter
            +takes_value
            "Filters by language, seperated by a comma. i.e. -t=Rust,C")
        /*(@arg verbose: -v --verbose ...
            "Set log output level:
            1: to show unknown file extensions,
            2: reserved for future debugging,
            3: enable file level trace. Not recommended on multiple files")*/
    ).get_matches();

    let loud = !matches.is_present("quiet");

    let filter: Option<Vec<_>> = matches.value_of("filter").map(|e| {
        e.split(",")
         .map(|t| t.parse::<LanguageType>())
         .filter_map(Result::ok)
         .collect()
    });

    let paths: Vec<&str> = matches.values_of("input")
                                  .map(|v| v.collect())
                                  .unwrap_or(vec!["."]);

    let mut code_has_changed = false;
    for path in paths {
        if repo_has_changes(path, &filter)? {
            code_has_changed = true;
            break;
        }
    }

    if code_has_changed {
        if loud {
            println!("Code has changed.");
        }

    } else {
        if loud {
            println!("No code has changed.");
        }
        process::exit(1)
    }

    Ok(())
}

fn repo_has_changes(path: &str, filter: &Option<Vec<LanguageType>>)
    -> Result<bool, Box<Error>>
{
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

    let diff_map = Mutex::new(HashMap::new());

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
        if changes.added.has_code_changes(filter)? {
            return Ok(true)
        }
    }

    repo.checkout_tree(previous.as_object(), None)?;

    for (_, changes) in diff_map.lock().unwrap().iter() {
        if changes.removed.has_code_changes(filter)? {
            return Ok(true)
        }
    }

    repo.checkout_tree(head.as_object(), None)?;

    Ok(false)
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

    macro_rules! add_and_commit {
        ($msg:expr) => {{
            git!("add" ".");
            git!("commit" "-q" "-m" $msg);
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

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("lib.rs"), b"Hello\nWorld")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Second Commit");

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

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("lib.rs"), b"// Hello\n// World")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("No code has changed.\n")
            .failure();
    }

    #[test]
    fn text_file_changed() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.md"), b"// Hello\n")
            .expect("Couldn't write to lib.md");

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("lib.md"), b"// Hello\n// World")
            .expect("Couldn't write to lib.md");

        add_and_commit!("Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("No code has changed.\n")
            .failure();
    }

    #[test]
    fn multi_line_comment_changed() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.md"), b"/* Hello\n\n*/\n")
            .expect("Couldn't write to lib.md");

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("lib.md"), b"/* Hello\nWorld\n*/\n")
            .expect("Couldn't write to lib.md");

        add_and_commit!("Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("No code has changed.\n")
            .failure();
    }

    #[test]
    fn code_between_multi_line_changed() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.rs"), b"/*Hello*/\nWorld\n/*!!!!*/\n")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("lib.rs"), b"/*Hello*/\nHello World\n/*!!!!*/\n")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("Code has changed.\n")
            .success();
    }

    #[test]
    fn code_in_example_block_changed() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.rs"), b"/// ```\nHello\nWorld\n/// ```\n")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("lib.rs"), b"/// ```\nHello\n/// ```\n")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("Code has changed.\n")
            .success();
    }

    #[test]
    fn comment_added_to_code() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.rs"), b"Hello\n")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("lib.rs"), b"Hello\n// World\n")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("No code has changed.\n")
            .failure();
    }

    #[test]
    fn new_code_file() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.rs"), b"Hello\n")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("lib.c"), b"Hello\n// World\n")
            .expect("Couldn't write to lib.c");

        add_and_commit!("Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("Code has changed.\n")
            .success();
    }

    #[test]
    fn new_doc_file() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.rs"), b"Hello\n")
            .expect("Couldn't write to lib.rs");

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("README.md"), b"Hello\nWorld\n")
            .expect("Couldn't write to README");

        add_and_commit!("Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&[temp.path()])
            .assert()
            .stdout("No code has changed.\n")
            .failure();
    }

    #[test]
    fn new_filtered_file() {
        let (current, temp) = create_and_init_tempdir();

        fs::write(temp.path().join("lib.rs"), b"Hello\n")
            .expect("Couldn't write to lib.rs");

        fs::write(temp.path().join("lib.c"), b"Hello\nWorld\n")
            .expect("Couldn't write to lib.c");

        add_and_commit!("Initial Commit");

        fs::write(temp.path().join("script.py"), b"Hello\nWorld\n")
            .expect("Couldn't write to script.py");

        add_and_commit!("Second Commit");

        env::set_current_dir(current).unwrap();

        Command::main_binary()
            .unwrap()
            .args(&["--filter", "Rust,C", temp.path().to_str().unwrap()])
            .assert()
            .stdout("No code has changed.\n")
            .failure();
    }

}
