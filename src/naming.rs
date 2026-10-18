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

#[cfg(test)]
mod tests {
    use super::*;

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
