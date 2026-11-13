//! Globs: patterns over relative paths, `io.fs`'s `Glob`. A pattern is
//! segments separated by `/`: `**` is any number of whole segments, and in
//! any other segment `*` matches within the segment, `?` one character, and
//! `[abc]` or `[a-z]` one character of the set. Matching reads text and
//! never the file system. See [HQL-0040](../doc/proposals/HQL-0040/README.md).

/// A compiled pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Glob {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    /// `**`: any number of whole segments, none included.
    Any,
    One(Vec<Token>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Literal(char),
    /// `*`: any run of characters within the segment.
    Star,
    /// `?`: one character.
    Question,
    /// `[...]`: one character within one of the ranges.
    Class(Vec<(char, char)>),
}

impl Glob {
    /// Compile `pattern`.
    ///
    /// # Errors
    /// Names what does not parse: an empty pattern, an unclosed or empty
    /// `[`, or a `**` that is not a whole segment.
    pub(crate) fn parse(pattern: &str) -> Result<Self, String> {
        if pattern.is_empty() {
            return Err("a glob pattern is not empty".to_owned());
        }
        let segments = pattern
            .split('/')
            .filter(|segment| !segment.is_empty() && *segment != ".")
            .map(|segment| segment_of(segment, pattern))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { segments })
    }

    /// Whether the path whose segments are `path` matches.
    pub(crate) fn matches<S: AsRef<str>>(&self, path: &[S]) -> bool {
        matched(&self.segments, path)
    }
}

/// The segments of a relative path as a glob reads them: separated by `/`,
/// with empty and `.` segments dropped.
pub(crate) fn segments(text: &str) -> Vec<&str> {
    text.split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect()
}

fn segment_of(segment: &str, pattern: &str) -> Result<Segment, String> {
    if segment == "**" {
        return Ok(Segment::Any);
    }
    if segment.contains("**") {
        return Err(format!(
            "{pattern:?} is not a glob: `**` stands for whole segments, alone between `/`"
        ));
    }
    let mut tokens = Vec::new();
    let mut characters = segment.chars();
    while let Some(character) = characters.next() {
        tokens.push(match character {
            '*' => Token::Star,
            '?' => Token::Question,
            '[' => {
                let mut ranges = Vec::new();
                let mut closed = false;
                let mut previous: Option<char> = None;
                let mut pending_range = false;
                for inner in characters.by_ref() {
                    match inner {
                        ']' => {
                            closed = true;
                            break;
                        }
                        '-' if previous.is_some() && !pending_range => pending_range = true,
                        _ if pending_range => {
                            let start = previous.take().unwrap_or(inner);
                            ranges.pop();
                            ranges.push((start, inner));
                            pending_range = false;
                        }
                        _ => {
                            ranges.push((inner, inner));
                            previous = Some(inner);
                        }
                    }
                }
                if pending_range {
                    ranges.push(('-', '-'));
                }
                if !closed {
                    return Err(format!("{pattern:?} is not a glob: a `[` is not closed"));
                }
                if ranges.is_empty() {
                    return Err(format!(
                        "{pattern:?} is not a glob: `[]` holds no character"
                    ));
                }
                Token::Class(ranges)
            }
            other => Token::Literal(other),
        });
    }
    Ok(Segment::One(tokens))
}

fn matched<S: AsRef<str>>(pattern: &[Segment], path: &[S]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((Segment::Any, rest)) => (0..=path.len()).any(|skip| matched(rest, &path[skip..])),
        Some((Segment::One(tokens), rest)) => path.split_first().is_some_and(|(first, after)| {
            let characters: Vec<char> = first.as_ref().chars().collect();
            within(tokens, &characters) && matched(rest, after)
        }),
    }
}

/// Whether one segment's characters match its tokens.
fn within(tokens: &[Token], characters: &[char]) -> bool {
    match tokens.split_first() {
        None => characters.is_empty(),
        Some((Token::Star, rest)) => {
            (0..=characters.len()).any(|skip| within(rest, &characters[skip..]))
        }
        Some((token, rest)) => characters.split_first().is_some_and(|(first, after)| {
            let one = match token {
                Token::Literal(literal) => literal == first,
                Token::Question => true,
                Token::Class(ranges) => ranges
                    .iter()
                    .any(|(low, high)| (*low..=*high).contains(first)),
                Token::Star => false,
            };
            one && within(rest, after)
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(pattern: &str, path: &str) -> bool {
        Glob::parse(pattern).unwrap().matches(&segments(path))
    }

    #[test]
    fn each_form_matches_as_stated() {
        // `*` within one segment.
        assert!(matches("*.csv", "a.csv"));
        assert!(!matches("*.csv", "data/a.csv"));
        assert!(matches("data/*.csv", "data/a.csv"));
        // `**` any number of whole segments, none included.
        assert!(matches("**/*.csv", "a.csv"));
        assert!(matches("**/*.csv", "x/y/a.csv"));
        assert!(matches("a/**", "a/b/c"));
        assert!(matches("a/**/z", "a/z"));
        assert!(!matches("**/*.csv", "a.json"));
        // `?` one character, never a `/`.
        assert!(matches("?.md", "a.md"));
        assert!(!matches("?.md", "ab.md"));
        assert!(!matches("a?b", "a/b"));
        // `[abc]` and `[a-z]` one character of the set.
        assert!(matches("[abc].md", "b.md"));
        assert!(!matches("[abc].md", "d.md"));
        assert!(matches("[a-c]x", "bx"));
        assert!(matches("[a-]x", "-x"));
    }

    #[test]
    fn a_pattern_that_does_not_parse_says_why() {
        for (pattern, says) in [
            ("[ab", "not closed"),
            ("[]", "holds no character"),
            ("a**b", "whole segments"),
            ("", "not empty"),
        ] {
            let refusal = Glob::parse(pattern).unwrap_err();
            assert!(refusal.contains(says), "{pattern}: {refusal}");
        }
    }
}
