//! The naming lint: each kind of name has one casing, whoever provides it.
//!
//! A name a program introduces that departs from its kind's convention is a
//! warning, never an error. See [HQL-0011](../../doc/proposals/HQL-0011/README.md).

/// The kinds of name the lint knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A type, concrete or abstract: `PascalCase`.
    Type,
    /// A type parameter: `PascalCase`.
    TypeParameter,
    /// A step, an operation or a function: `snake_case`, with no `get_`.
    Function,
    /// A binding: `snake_case`.
    Binding,
    /// A parameter: `snake_case`.
    Parameter,
    /// A declared field: `snake_case`.
    Field,
    /// An extension: one lowercase word.
    Extension,
}

impl Kind {
    const fn noun(self) -> &'static str {
        match self {
            Self::Type => "a type",
            Self::TypeParameter => "a type parameter",
            Self::Function => "a function",
            Self::Binding => "a binding",
            Self::Parameter => "a parameter",
            Self::Field => "a field",
            Self::Extension => "an extension",
        }
    }
}

/// Why a name departs from its kind's convention, or `None` when it does not.
///
/// "Uppercase" is the Unicode property, so a name in a script without case
/// passes every rule.
#[must_use]
pub fn departure(kind: Kind, name: &str) -> Option<String> {
    let first = name.chars().next()?;
    let upper = name.chars().any(char::is_uppercase);
    let expected = match kind {
        Kind::Type | Kind::TypeParameter if first.is_lowercase() || name.contains('_') => {
            "PascalCase"
        }
        Kind::Function if name.starts_with("get_") => {
            return Some(format!(
                "`{name}` is a function, named for the value it returns, without a `get_` prefix"
            ));
        }
        Kind::Function | Kind::Binding | Kind::Parameter | Kind::Field if upper => "snake_case",
        Kind::Extension if upper || name.contains('_') => "one lowercase word",
        _ => return None,
    };
    Some(format!(
        "`{name}` is {}, written in {expected}",
        kind.noun()
    ))
}

/// The name written in its kind's convention, when the lint departs from it
/// and the conventional spelling differs.
#[must_use]
pub fn conventional(kind: Kind, name: &str) -> Option<String> {
    departure(kind, name)?;
    let words = words(name);
    let written = match kind {
        Kind::Type | Kind::TypeParameter => words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(chars).collect()
                })
            })
            .collect(),
        Kind::Extension => words.concat(),
        Kind::Function | Kind::Binding | Kind::Parameter | Kind::Field => {
            let words = match words.split_first() {
                Some((first, rest)) if kind == Kind::Function && first == "get" => rest.to_vec(),
                _ => words,
            };
            words.join("_")
        }
    };
    (!written.is_empty() && written != name).then_some(written)
}

/// The lowercase words of a name: split at `_` and before each uppercase
/// letter that follows a lowercase one.
fn words(name: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut previous_lower = false;
    for c in name.chars() {
        if c == '_' {
            words.push(String::new());
            previous_lower = false;
            continue;
        }
        if words.is_empty() || (c.is_uppercase() && previous_lower) {
            words.push(String::new());
        }
        previous_lower = c.is_lowercase();
        if let Some(last) = words.last_mut() {
            last.extend(c.to_lowercase());
        }
    }
    words.into_iter().filter(|word| !word.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_conventional_spelling_is_offered() {
        assert_eq!(
            conventional(Kind::Function, "totalOf").as_deref(),
            Some("total_of")
        );
        assert_eq!(
            conventional(Kind::Type, "hmd_card").as_deref(),
            Some("HmdCard")
        );
        assert_eq!(
            conventional(Kind::Function, "get_size").as_deref(),
            Some("size")
        );
        assert_eq!(conventional(Kind::Binding, "bird_cards"), None);
    }

    #[test]
    fn each_kind_has_one_convention() {
        assert!(departure(Kind::Binding, "bird_cards").is_none());
        assert!(departure(Kind::Binding, "birdCards").is_some());
        assert!(departure(Kind::Type, "HmdCard").is_none());
        assert!(departure(Kind::Type, "hmd_card").is_some());
        assert!(departure(Kind::Type, "Hmd_Card").is_some());
        assert!(departure(Kind::TypeParameter, "T").is_none());
        assert!(
            departure(Kind::Function, "get_size")
                .unwrap()
                .contains("get_")
        );
        assert!(departure(Kind::Function, "get").is_none());
        assert!(departure(Kind::Extension, "semantic").is_none());
        assert!(departure(Kind::Extension, "my_ext").is_some());
        // A script without case passes.
        assert!(departure(Kind::Type, "名前").is_none());
        assert!(departure(Kind::Binding, "名前").is_none());
    }
}
