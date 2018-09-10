use std::fs::{self, File};
use std::io::Read;
use std::error::Error;
use std::path::PathBuf;

use encoding_rs_io::DecodeReaderBytes;
use git2::{Delta, DiffDelta};
use regex::{self, RegexSet};
use tokei::LanguageType;

#[derive(Debug)]
pub struct Changes {
    pub removed: FileChanges,
    pub added: FileChanges,
}

#[derive(Debug)]
pub struct FileChanges {
    pub lines: Vec<u32>,
    path: Option<PathBuf>,
    status: Delta,
}

impl Changes {
    pub fn new(delta: &DiffDelta) -> Self {
        let removed = FileChanges {
            lines: Vec::new(),
            path: delta.old_file().path().and_then(|p| fs::canonicalize(p).ok()),
            status: delta.status(),
        };

        let added = FileChanges {
            lines: Vec::new(),
            path: delta.new_file().path().and_then(|p| fs::canonicalize(p).ok()),
            status: delta.status(),
        };

        Self {
            removed,
            added,
        }
    }
}

impl FileChanges {
    pub fn has_code_changes(&self) -> Result<bool, Box<Error>> {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => {println!("No path."); return Ok(false) }
        };

        let language = match LanguageType::from_path(&path) {
            Some(l) => l,
            None => {println!("Unknown language."); return Ok(true) }
        };

        let multi_line_comments: Vec<_> = language.multi_line_comments()
            .into_iter()
            .chain(language.nested_comments())
            .collect();

        let text = {
            let f = File::open(&path)?;
            let mut s = String::new();
            let mut reader = DecodeReaderBytes::new(f);

            reader.read_to_string(&mut s)?;
            s
        };

        for line_num in &self.lines {
            let line = text.lines().skip(*line_num as usize - 1).next().unwrap();
            let escaped = regex::escape(&line);
            let mut regexes = Vec::with_capacity(1);
            let mut starts_with_comment = false;

            // Regexes that check if it's code.
            for comment in language.line_comments() {
                starts_with_comment = starts_with_comment ||
                    line.starts_with(comment);

                regexes.push(format!(
                        "^{comment} ```.*{text}.*\\n{comment} ```",
                        comment = regex::escape(comment),
                        text = escaped,
                    )
                );
            }

            for (start, end) in &multi_line_comments {
                regexes.push(format!(
                        "{start} ```.*{text}.*```.*{end}",
                        start = regex::escape(start),
                        text = escaped,
                        end = regex::escape(end),
                    )
                );
            }

            if language == LanguageType::Rust {
                regexes.push(format!(
                        "^/// ```.*{text}.*\\n/// ```",
                        text = escaped,
                    )
                );
            }

            // Regexes that check if it's comments.
            let is_in_comments = regexes.len();
            for (start, end) in &multi_line_comments {
                regexes.push(format!(
                        "{start}.*{text}.*{end}",
                        start = regex::escape(start),
                        text = escaped,
                        end = regex::escape(end),
                    )
                );
            }

            let matches = RegexSet::new(&regexes)?.matches(&text);


            if (matches.iter().any(|x| x < is_in_comments) ||
                !matches.matched_any()) &&
                !starts_with_comment
            {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
