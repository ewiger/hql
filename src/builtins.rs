//! The suggestion index over every registered step name.
//!
//! The names themselves live with the extensions that provide them, in
//! [`crate::extensions`]. What stays here is the question a diagnostic asks:
//! is this a step at all, and if not, which step was probably meant. The
//! search covers every *registered* extension rather than only the imported
//! ones, so a missing import reports the import rather than a misspelling.

use crate::extensions;

/// Whether a name is a step any registered extension provides.
#[must_use]
pub fn exists(name: &str) -> bool {
    extensions::names().any(|step| step == name)
}

/// The closest step name by edit distance, for a "did you mean" hint.
#[must_use]
pub fn nearest(name: &str) -> Option<&'static str> {
    extensions::names()
        .map(|step| (distance(name, step), step))
        .filter(|(distance, _)| *distance <= 2)
        // A tie is broken by name, so a suggestion never depends on the
        // order the registry happens to be written in.
        .min_by_key(|(distance, name)| (*distance, *name))
        .map(|(_, name)| name)
}

fn distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (row, from) in left.chars().enumerate() {
        let mut current = vec![row + 1];
        for (column, to) in right.iter().enumerate() {
            let cost = usize::from(from != *to);
            current.push(
                (previous[column] + cost)
                    .min(previous[column + 1] + 1)
                    .min(current[column] + 1),
            );
        }
        previous = current;
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_a_near_miss_and_stays_quiet_otherwise() {
        assert_eq!(nearest("tabel"), Some("table"));
        assert_eq!(nearest("lexicl"), Some("lexical"));
        assert_eq!(nearest("qwertyuiop"), None);
    }

    #[test]
    fn every_registered_step_exists() {
        for name in extensions::names() {
            assert!(exists(name), "{name}");
        }
        assert!(!exists("nonesuch"));
    }
}
