//! The names a pipeline may use, and what each one is for.
//!
//! These are ordinary functions rather than grammar. Keeping the list in one
//! place is what lets `hql builtins` and the type checker agree about it.

/// One name a program may call.
#[derive(Debug, Clone, Copy)]
pub struct Builtin {
    /// How it is written.
    pub name: &'static str,
    /// How it is called, for help output.
    pub signature: &'static str,
    /// One sentence about what it does.
    pub summary: &'static str,
}

/// Every builtin, in the order help prints them.
pub const BUILTINS: &[Builtin] = &[
    Builtin {
        name: "count",
        signature: "collection | count",
        summary: "How many elements a collection holds.",
    },
    Builtin {
        name: "sort",
        signature: "collection | sort  |  collection | sort(by = c => c.title)",
        summary: "Order a collection by its elements' own key, or by one you supply.",
    },
    Builtin {
        name: "take",
        signature: "collection | take(5)",
        summary: "The first n elements, in the elements' own order.",
    },
    Builtin {
        name: "filter",
        signature: "collection | filter(c => c.kind)",
        summary: "Keep the elements a predicate accepts.",
    },
    Builtin {
        name: "map",
        signature: "collection | map(c => c.title)",
        summary: "Apply a function to every element, yielding a sequence.",
    },
    Builtin {
        name: "typed",
        signature: "cards | typed(RelationCard)",
        summary: "Keep the elements that are of a type, checked rather than trusted.",
    },
    Builtin {
        name: "uplinks",
        signature: "card | uplinks",
        summary: "The edges leaving a card.",
    },
    Builtin {
        name: "downlinks",
        signature: "card | downlinks",
        summary: "The edges entering a card, which needs a vault.",
    },
    Builtin {
        name: "semantic",
        signature: "cards | semantic(\"bearer token authorization\")",
        summary: "Rank a corpus against a query, retaining the retrieval behind every score.",
    },
    Builtin {
        name: "expand",
        signature: "collection | expand(depth = 1)",
        summary: "Traverse outwards, keeping why each node is present.",
    },
    Builtin {
        name: "graph",
        signature: "collection | graph",
        summary: "Project a collection of cards into a graph.",
    },
    Builtin {
        name: "table",
        signature: "value | table",
        summary: "Render as a table. Terminal: nothing downstream reads it.",
    },
    Builtin {
        name: "json",
        signature: "value | json",
        summary: "Render as JSON. Terminal.",
    },
    Builtin {
        name: "text",
        signature: "value | text",
        summary: "Render as plain text. Terminal.",
    },
];

/// Whether a name is a builtin.
#[must_use]
pub fn exists(name: &str) -> bool {
    BUILTINS.iter().any(|builtin| builtin.name == name)
}

/// The builtin with a name.
#[must_use]
pub fn find(name: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|builtin| builtin.name == name)
}

/// The closest builtin name by edit distance, for a "did you mean" hint.
#[must_use]
pub fn nearest(name: &str) -> Option<&'static str> {
    BUILTINS
        .iter()
        .map(|builtin| (distance(name, builtin.name), builtin.name))
        .filter(|(distance, _)| *distance <= 2)
        // A tie is broken by name, so a suggestion never depends on the
        // order this list happens to be written in.
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
        assert_eq!(nearest("semantik"), Some("semantic"));
        assert_eq!(nearest("qwertyuiop"), None);
    }
}
