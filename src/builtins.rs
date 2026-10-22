//! The suggestion index over every registered step name, and over the
//! combiners a fold names.
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

/// Names that were steps and are not, with the step that replaces each.
const RETIRED: &[(&str, &str)] = &[("typed", "of_type"), ("flatten", "emit")];

/// Names that were steps and have no single replacement, with what to write
/// instead.
const EXPLAINED: &[(&str, &str)] = &[(
    "table",
    "a table is a value of the type `Table`, built by its constructor: \
     `Table(xs, columns = [...])`",
)];

/// The hint a diagnostic appends for a name that is not a step: what to write
/// instead, or the closest name.
#[must_use]
pub fn hint(name: &str) -> String {
    if let Some((_, explanation)) = EXPLAINED.iter().find(|(retired, _)| *retired == name) {
        return format!("; {explanation}");
    }
    if crate::constructors::named(name).is_some() {
        return format!(
            "; `{name}` is a constructor: call it, `{name}(xs)`, or pass it to a step that takes \
             a function, `map({name})`"
        );
    }
    nearest(name)
        .map(|near| format!("; did you mean `{near}`?"))
        .unwrap_or_default()
}

/// The closest step name by edit distance, for a "did you mean" hint. A
/// retired name suggests the step that replaced it.
#[must_use]
pub fn nearest(name: &str) -> Option<&'static str> {
    if let Some((_, replacement)) = RETIRED.iter().find(|(retired, _)| *retired == name) {
        return Some(replacement);
    }
    // A lowercase spelling of a type names its constructor: `list` is `List`.
    let mut letters = name.chars();
    if let Some(first) = letters.next() {
        let capitalised = first.to_uppercase().chain(letters).collect::<String>();
        if let Some(constructor) = crate::constructors::named(&capitalised) {
            return Some(constructor.0);
        }
    }
    extensions::names()
        .map(|step| (distance(name, step), step))
        .filter(|(distance, _)| *distance <= 2)
        // A tie is broken by name, so a suggestion never depends on the
        // order the registry happens to be written in.
        .min_by_key(|(distance, name)| (*distance, *name))
        .map(|(_, name)| name)
}

/// The hint for a name a fold's `by` position does not resolve: the
/// closest combiner, or the list of them. A fold names a combiner or a
/// function, so a step or a math function is no suggestion there.
#[must_use]
pub fn combiner_hint(name: &str) -> String {
    let near = extensions::COMBINERS
        .iter()
        .filter(|combiner| combiner.name.chars().all(char::is_alphabetic))
        .map(|combiner| (distance(name, combiner.name), combiner.name))
        .filter(|(distance, _)| *distance <= 2)
        .min_by_key(|(distance, name)| (*distance, *name));
    if let Some((_, combiner)) = near {
        return format!("; a fold names a combiner or a function: did you mean `{combiner}`?");
    }
    let all: Vec<String> = extensions::COMBINERS
        .iter()
        .map(|combiner| format!("`{}`", combiner.name))
        .collect();
    format!(
        "; a fold names a function or one of the combiners {}",
        all.join(", ")
    )
}

pub(crate) fn distance(left: &str, right: &str) -> usize {
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
        assert_eq!(nearest("tabel"), Some("take"));
        assert_eq!(nearest("lexicl"), Some("lexical"));
        assert_eq!(nearest("qwertyuiop"), None);
    }

    #[test]
    fn a_fold_suggests_among_the_combiners_only() {
        assert!(combiner_hint("mn").contains("`min`"));
        assert!(combiner_hint("merg").contains("`merge`"));
        let listed = combiner_hint("foo");
        assert!(!listed.contains("cos"), "{listed}");
        assert!(
            listed.contains("`+`") && listed.contains("`concat`"),
            "{listed}"
        );
    }

    #[test]
    fn every_registered_step_exists() {
        for name in extensions::names() {
            assert!(exists(name), "{name}");
        }
        assert!(!exists("nonesuch"));
    }
}
