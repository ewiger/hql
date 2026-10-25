//! The HQL book runs: every `hql` block in `doc/public/learn/` is a program,
//! and the `text` block after it is what `hql` writes for it.
//!
//! Directives, on a line before a block:
//! `<!-- define: NAME -->` names the block for later ones;
//! `<!-- run: skip -->` leaves it unrun;
//! `<!-- run: [check] [with A,B] [ARGS…] -->` runs `hql check` instead of
//! `eval`, prefixes the named blocks, and replaces the default arguments,
//! `--vault examples/tutorial` (`novault` runs with none).
//! A block inside a four-backtick fence is an example document, not a program.

use std::collections::HashMap;
use std::process::Command;

struct Block {
    program: String,
    directive: Option<String>,
    expected: Option<String>,
}

fn blocks(text: &str, defined: &mut HashMap<String, String>) -> Vec<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let (mut directive, mut name) = (None, None);
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        if line.starts_with("````") {
            index += 1;
            while !lines[index].starts_with("````") {
                index += 1;
            }
            index += 1;
            continue;
        }
        if let Some(rest) = line
            .strip_prefix("<!-- run: ")
            .and_then(|rest| rest.strip_suffix(" -->"))
        {
            directive = Some(rest.to_owned());
        } else if let Some(rest) = line
            .strip_prefix("<!-- define: ")
            .and_then(|rest| rest.strip_suffix(" -->"))
        {
            name = Some(rest.to_owned());
        } else if line == "```hql" {
            let end = index
                + 1
                + lines[index + 1..]
                    .iter()
                    .position(|line| line.trim() == "```")
                    .unwrap();
            let program = lines[index + 1..end].join("\n");
            if let Some(name) = name.take() {
                defined.insert(name, program.clone());
            }
            let mut next = end + 1;
            while next < lines.len() && lines[next].trim().is_empty() {
                next += 1;
            }
            let expected = (next < lines.len() && lines[next].trim() == "```text").then(|| {
                let close = next
                    + 1
                    + lines[next + 1..]
                        .iter()
                        .position(|line| line.trim() == "```")
                        .unwrap();
                lines[next + 1..close].join("\n")
            });
            found.push(Block {
                program,
                directive: directive.take(),
                expected,
            });
            index = end;
        }
        index += 1;
    }
    found
}

fn run(block: &Block, defined: &HashMap<String, String>) -> String {
    let mut words: Vec<&str> = block
        .directive
        .as_deref()
        .map(|d| d.split_whitespace().collect())
        .unwrap_or_default();
    let mut command = "eval";
    if words.first() == Some(&"check") {
        command = "check";
        words.remove(0);
    }
    let mut source = String::new();
    if words.first() == Some(&"with") {
        for name in words[1].split(',') {
            source.push_str(&defined[name]);
            source.push('\n');
        }
        words.drain(..2);
    }
    source.push_str(&block.program);
    let arguments: Vec<&str> = match words.as_slice() {
        [] => vec!["--vault", "examples/tutorial"],
        ["novault"] => vec![],
        rest => rest.to_vec(),
    };
    let mut hql = Command::new(env!("CARGO_BIN_EXE_hql"));
    hql.args(&arguments);
    let output = if command == "check" {
        use std::io::Write;
        let mut child = hql
            .args(["check", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    } else {
        hql.args(["eval", &source]).output().unwrap()
    };
    let text = if output.status.success() {
        output.stdout
    } else {
        output.stderr
    };
    // CSV ends its rows with `\r\n`, which a page shows as a line break.
    String::from_utf8(text)
        .unwrap()
        .replace("\r\n", "\n")
        .trim_end()
        .to_owned()
}

#[test]
fn every_program_in_the_book_writes_what_the_book_shows() {
    // A chapter may be a folder of pages: `02-data-types/` holds one per type.
    fn pages(directory: &std::path::Path, into: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pages(&path, into);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                into.push(path);
            }
        }
    }
    let mut chapters = Vec::new();
    pages(std::path::Path::new("doc/public/learn"), &mut chapters);
    chapters.sort();
    let mut checked = 0;
    for chapter in chapters {
        let text = std::fs::read_to_string(&chapter).unwrap();
        let mut defined = HashMap::new();
        for block in blocks(&text, &mut defined) {
            if block.directive.as_deref() == Some("skip") {
                continue;
            }
            let written = run(&block, &defined);
            if let Some(expected) = &block.expected {
                assert_eq!(
                    &written,
                    expected,
                    "{}: the program\n{}\nwrites\n{written}",
                    chapter.display(),
                    block.program
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 50, "only {checked} programs found");
}

#[test]
fn the_notebook_the_book_shows_builds() {
    let output = Command::new(env!("CARGO_BIN_EXE_hql"))
        .args([
            "--vault",
            "examples/tutorial",
            "render",
            "examples/notebook/reading-list.hmd",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let built = String::from_utf8(output.stdout).unwrap();
    let chapter = std::fs::read_to_string("doc/public/learn/12-documents.md").unwrap();
    assert!(
        chapter.contains(built.trim_end()),
        "chapter 12 shows a different build:\n{built}"
    );
}
