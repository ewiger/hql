//! The schema boundary of a configured source (HQL-0035): the resolver that
//! merges a format's defaults, a source's configuration and an invocation's
//! override into the one effective boundary I/O proceeds with, each
//! property keeping the layer it came from.

use crate::sources::{Absence, External, FieldMapping};
use crate::types::TypeRef;
use std::collections::BTreeMap;

/// One field of the element type a boundary maps: its name, its type with
/// optionality taken off, and whether it is written `f?`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclaredField {
    /// The HQL field's name.
    pub name: String,
    /// Its type; an optional field's present type.
    pub ty: TypeRef,
    /// Whether the field is optional.
    pub optional: bool,
}

/// The layer a boundary property comes from, earliest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// The format's default.
    Format,
    /// The source's `[sources.<id>.fields]`.
    Configuration,
    /// An invocation's override.
    Invocation,
}

impl Origin {
    /// The word an inspection prints for the layer.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Format => "format",
            Self::Configuration => "configuration",
            Self::Invocation => "invocation",
        }
    }
}

/// A resolved property and the layer it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Held<T> {
    /// The property's value.
    pub value: T,
    /// The layer that wrote it.
    pub origin: Origin,
}

/// One HQL field of an effective boundary and every external property of it.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveField {
    /// The HQL field.
    pub field: DeclaredField,
    /// The external field it is held in.
    pub external: Held<External>,
    /// The external type, in the format's vocabulary.
    pub external_type: Held<String>,
    /// Its place among the external fields, from 1.
    pub position: Held<usize>,
    /// Whether it is read from the source.
    pub readable: Held<bool>,
    /// Whether it is written to a target.
    pub writable: Held<bool>,
    /// The external representations an optional field reads as absent.
    pub absent: Held<Vec<Absence>>,
    /// Whether a representation that may not keep the value is permitted.
    pub lossy: Held<bool>,
    /// The keys the kind's extension owns.
    pub owned: BTreeMap<String, Held<toml::Value>>,
}

/// The boundary a source's I/O proceeds with: every field of its element
/// type, in declaration order.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveBoundary {
    /// The source's id.
    pub source: String,
    /// The source's kind, `csv`.
    pub kind: String,
    /// The fields, in the order the element type declares them.
    pub fields: Vec<EffectiveField>,
    /// What a read does with an external field no HQL field reads.
    pub strictness: crate::sources::Strictness,
}

impl EffectiveBoundary {
    /// The effective field of an HQL field.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&EffectiveField> {
        self.fields.iter().find(|held| held.field.name == name)
    }
}

/// The kinds of boundary error (HQL-0035).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The configuration maps a field the element type does not declare.
    UndeclaredField,
    /// A required field's external field is not among the external fields.
    UnmappedField,
    /// Two HQL fields read one external field.
    SharedExternalField,
    /// A configured external field is not among the external fields.
    MissingExternalField,
    /// The external type is not the format's, or does not decode to the
    /// field's type.
    NoDecoder,
    /// The external type may not keep the field's value, and the field does
    /// not permit it.
    LossyRepresentation,
}

impl ErrorKind {
    /// The kind as a diagnostic names it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::UndeclaredField => "undeclared field",
            Self::UnmappedField => "unmapped field",
            Self::SharedExternalField => "shared external field",
            Self::MissingExternalField => "missing external field",
            Self::NoDecoder => "no decoder",
            Self::LossyRepresentation => "lossy representation",
        }
    }
}

/// One boundary error: what is wrong, of which field, and from which layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryError {
    /// What kind of error it is.
    pub kind: ErrorKind,
    /// The source's id.
    pub source: String,
    /// The HQL field at fault.
    pub field: String,
    /// The external field at fault, where there is one.
    pub external: Option<String>,
    /// The layer of the property at fault.
    pub origin: Origin,
    /// The error as a sentence.
    pub message: String,
}

impl std::fmt::Display for BoundaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "source `{}`, field `{}`: {} ({}, from the {})",
            self.source,
            self.field,
            self.message,
            self.kind.name(),
            self.origin.label()
        )
    }
}

/// The external field as a diagnostic and a header name it.
#[must_use]
pub fn written(external: &External) -> String {
    match external {
        External::Name(name) => name.clone(),
        External::Path(segments) => segments.join("."),
    }
}

/// The layers of one boundary, earliest first after the format's defaults.
#[derive(Debug, Clone, Copy)]
pub struct Layers<'a> {
    /// The source's configured fields.
    pub configuration: &'a BTreeMap<String, FieldMapping>,
    /// An invocation's override.
    pub invocation: &'a BTreeMap<String, FieldMapping>,
}

/// Resolve a source's boundary for `fields`, the element type's, against
/// `external`, the external fields where the format knows them before
/// reading. Every error is reported, not only the first.
///
/// # Errors
///
/// Returns every boundary error, each naming the field and the layer at
/// fault.
pub fn resolve(
    source: &str,
    kind: &str,
    fields: &[DeclaredField],
    layers: Layers<'_>,
    external: Option<&[String]>,
) -> Result<EffectiveBoundary, Vec<BoundaryError>> {
    let mut errors = Vec::new();
    let error = |kind_of: ErrorKind,
                 field: &str,
                 outside: Option<String>,
                 origin: Origin,
                 message: String| BoundaryError {
        kind: kind_of,
        source: source.to_owned(),
        field: field.to_owned(),
        external: outside,
        origin,
        message,
    };
    for (layer, origin) in [
        (layers.configuration, Origin::Configuration),
        (layers.invocation, Origin::Invocation),
    ] {
        for name in layer.keys() {
            if !fields.iter().any(|field| field.name == *name) {
                errors.push(error(
                    ErrorKind::UndeclaredField,
                    name,
                    None,
                    origin,
                    "the element type declares no such field".to_owned(),
                ));
            }
        }
    }
    let mut resolved = Vec::with_capacity(fields.len());
    for (index, field) in fields.iter().enumerate() {
        let written_layers = [
            (layers.configuration.get(&field.name), Origin::Configuration),
            (layers.invocation.get(&field.name), Origin::Invocation),
        ];
        let mut owned = BTreeMap::new();
        for (mapping, origin) in &written_layers {
            for (key, value) in mapping.iter().flat_map(|mapping| &mapping.owned) {
                owned.insert(
                    key.clone(),
                    Held {
                        value: value.clone(),
                        origin: *origin,
                    },
                );
            }
        }
        resolved.push(EffectiveField {
            external: pick(
                &written_layers,
                External::Name(field.name.clone()),
                |mapping| mapping.external.clone(),
            ),
            external_type: pick(
                &written_layers,
                default_type(kind, &field.ty).to_owned(),
                |mapping| mapping.external_type.clone(),
            ),
            position: pick(&written_layers, index + 1, |mapping| mapping.position),
            readable: pick(&written_layers, true, |mapping| mapping.readable),
            writable: pick(&written_layers, true, |mapping| mapping.writable),
            absent: pick(&written_layers, default_absent(kind), |mapping| {
                mapping.absent.clone()
            }),
            lossy: pick(&written_layers, false, |mapping| mapping.lossy),
            owned,
            field: field.clone(),
        });
    }
    for held in &resolved {
        let name = &held.field.name;
        let outside = written(&held.external.value);
        match decodes(kind, &held.external_type.value, &held.field.ty) {
            Some(true) => {}
            Some(false) => errors.push(error(
                ErrorKind::NoDecoder,
                name,
                Some(outside.clone()),
                held.external_type.origin,
                format!(
                    "`{kind}`'s `{}` does not decode to {}",
                    held.external_type.value, held.field.ty
                ),
            )),
            None => errors.push(error(
                ErrorKind::NoDecoder,
                name,
                Some(outside.clone()),
                held.external_type.origin,
                format!("`{kind}` has no type `{}`", held.external_type.value),
            )),
        }
        if !held.lossy.value && lossy(kind, &held.external_type.value, &held.field.ty) {
            errors.push(error(
                ErrorKind::LossyRepresentation,
                name,
                Some(outside.clone()),
                held.external_type.origin,
                format!(
                    "`{kind}`'s `{}` may not keep every {}; write `lossy = true` to permit it",
                    held.external_type.value, held.field.ty
                ),
            ));
        }
        if let Some(known) = external
            && held.readable.value
            && !known.contains(&outside)
        {
            if held.external.origin != Origin::Format {
                errors.push(error(
                    ErrorKind::MissingExternalField,
                    name,
                    Some(outside.clone()),
                    held.external.origin,
                    format!("the source has no field `{outside}`"),
                ));
            } else if !held.field.optional {
                errors.push(error(
                    ErrorKind::UnmappedField,
                    name,
                    Some(outside.clone()),
                    held.external.origin,
                    format!(
                        "the field is required, and the source has no field `{outside}`; map \
                         it with `name`"
                    ),
                ));
            }
        }
        if held.readable.value
            && let Some(first) = resolved
                .iter()
                .find(|other| other.readable.value && written(&other.external.value) == outside)
            && first.field.name != *name
        {
            errors.push(error(
                ErrorKind::SharedExternalField,
                name,
                Some(outside.clone()),
                held.external.origin,
                format!(
                    "`{outside}` is read by `{}` already; each external field is read by one \
                     HQL field",
                    first.field.name
                ),
            ));
        }
    }
    if errors.is_empty() {
        Ok(EffectiveBoundary {
            source: source.to_owned(),
            kind: kind.to_owned(),
            fields: resolved,
            strictness: crate::sources::Strictness::default(),
        })
    } else {
        Err(errors)
    }
}

/// A property's value from the latest layer that writes it, the format's
/// `default` where none does.
fn pick<T>(
    layers: &[(Option<&FieldMapping>, Origin)],
    default: T,
    read: impl Fn(&FieldMapping) -> Option<T>,
) -> Held<T> {
    layers.iter().fold(
        Held {
            value: default,
            origin: Origin::Format,
        },
        |held, (mapping, origin)| match mapping.and_then(&read) {
            Some(value) => Held {
                value,
                origin: *origin,
            },
            None => held,
        },
    )
}

/// What an HQL type is, as the boundary tables tell types apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Int,
    Float,
    Decimal,
    Text,
    Bool,
    List,
    Object,
    Data,
    /// Any other scalar, a `Date`, carried as text.
    Other,
}

fn class(ty: &TypeRef) -> Class {
    if *ty == TypeRef::DATA {
        return Class::Data;
    }
    match ty {
        _ if *ty == TypeRef::INT => Class::Int,
        _ if *ty == TypeRef::FLOAT => Class::Float,
        _ if *ty == TypeRef::DECIMAL => Class::Decimal,
        _ if *ty == TypeRef::STR => Class::Text,
        _ if *ty == TypeRef::BOOL => Class::Bool,
        _ if ty.is_sequence() => Class::List,
        _ if crate::types::MapKind::of(ty.constructor).is_some()
            || crate::types::program::record(ty.constructor.0).is_some() =>
        {
            Class::Object
        }
        _ => Class::Other,
    }
}

/// The three families of format the tables distinguish.
fn family(kind: &str) -> &'static str {
    match kind {
        "csv" => "csv",
        "sqlite" => "sqlite",
        _ => "tree",
    }
}

/// The format's type for an HQL type: the first that decodes to it.
fn default_type(kind: &str, ty: &TypeRef) -> &'static str {
    match (family(kind), class(ty)) {
        ("csv", _) => "text",
        ("sqlite", Class::Int | Class::Bool) => "INTEGER",
        ("sqlite", Class::Float) => "REAL",
        ("sqlite", _) => "TEXT",
        (_, Class::Decimal) => "string",
        (_, Class::Int | Class::Float) => "number",
        (_, Class::Bool) => "boolean",
        (_, Class::List) => "array",
        (_, Class::Object | Class::Data) => "object",
        (_, Class::Text | Class::Other) => "string",
    }
}

/// What reads as an absent optional field unless the boundary says.
fn default_absent(kind: &str) -> Vec<Absence> {
    match family(kind) {
        "csv" => vec![Absence::Empty, Absence::Missing],
        "sqlite" => vec![Absence::Null],
        _ => vec![Absence::Missing, Absence::Null],
    }
}

/// Whether the format's external type may not keep every value of the HQL
/// type: a `Decimal` as a JSON number, a CSV float, or a SQLite `REAL` or
/// `NUMERIC`.
fn lossy(kind: &str, external: &str, ty: &TypeRef) -> bool {
    class(ty) == Class::Decimal
        && match family(kind) {
            "csv" => external == "float",
            "sqlite" => matches!(external.to_ascii_uppercase().as_str(), "REAL" | "NUMERIC"),
            _ => external == "number",
        }
}

/// Whether the format's external type decodes to the HQL type; `None`
/// where the format has no such type.
fn decodes(kind: &str, external: &str, ty: &TypeRef) -> Option<bool> {
    use Class::{Bool, Data, Decimal, Float, Int, List, Object, Other, Text};
    let target = class(ty);
    let into: &[Class] = match family(kind) {
        "csv" => match external {
            "text" => &[Int, Float, Decimal, Text, Bool, Other],
            "integer" => &[Int, Float, Decimal],
            "float" => &[Float, Decimal],
            "decimal" => &[Decimal],
            "boolean" => &[Bool],
            _ => return None,
        },
        "sqlite" => match external.to_ascii_uppercase().as_str() {
            "INTEGER" => &[Int, Float, Decimal, Bool],
            "REAL" => &[Float, Decimal],
            "TEXT" => &[Text, Decimal, Other],
            "NUMERIC" => &[Decimal, Int, Float],
            "BLOB" => &[Text],
            _ => return None,
        },
        _ => match external {
            "string" => &[Text, Decimal, Other],
            "number" => &[Int, Float, Decimal],
            "boolean" => &[Bool],
            "array" => &[List],
            "object" => &[Object],
            _ => return None,
        },
    };
    Some(target == Data || into.contains(&target))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(name: &str, ty: TypeRef, optional: bool) -> DeclaredField {
        DeclaredField {
            name: name.to_owned(),
            ty,
            optional,
        }
    }

    fn mapping(external: &str) -> FieldMapping {
        FieldMapping {
            external: Some(External::Name(external.to_owned())),
            ..FieldMapping::default()
        }
    }

    fn user_row() -> Vec<DeclaredField> {
        vec![
            declared("id", TypeRef::INT, false),
            declared("name", TypeRef::STR, false),
            declared("balance", TypeRef::DECIMAL, true),
        ]
    }

    #[test]
    fn the_format_supplies_every_property_nothing_overrides() {
        let none = BTreeMap::new();
        let layers = Layers {
            configuration: &none,
            invocation: &none,
        };
        let boundary = resolve("users", "sqlite", &user_row(), layers, None).unwrap();
        let balance = boundary.field("balance").unwrap();
        assert_eq!(balance.external.value, External::Name("balance".to_owned()));
        assert_eq!(balance.external_type.value, "TEXT");
        assert_eq!(balance.position.value, 3);
        assert_eq!(balance.absent.value, vec![Absence::Null]);
        assert!(
            boundary
                .fields
                .iter()
                .all(|field| { field.external.origin == Origin::Format && field.readable.value })
        );
    }

    #[test]
    fn each_property_takes_the_latest_layer_that_writes_it() {
        let configuration = BTreeMap::from([(
            "balance".to_owned(),
            FieldMapping {
                external_type: Some("NUMERIC".to_owned()),
                ..mapping("account_balance")
            },
        )]);
        let invocation = BTreeMap::from([(
            "balance".to_owned(),
            FieldMapping {
                external_type: Some("TEXT".to_owned()),
                ..FieldMapping::default()
            },
        )]);
        let layers = Layers {
            configuration: &configuration,
            invocation: &invocation,
        };
        let boundary = resolve("users", "sqlite", &user_row(), layers, None).unwrap();
        let balance = boundary.field("balance").unwrap();
        assert_eq!(
            balance.external,
            Held {
                value: External::Name("account_balance".to_owned()),
                origin: Origin::Configuration
            }
        );
        assert_eq!(
            balance.external_type,
            Held {
                value: "TEXT".to_owned(),
                origin: Origin::Invocation
            }
        );
        assert_eq!(balance.position.origin, Origin::Format);
    }

    #[test]
    fn every_boundary_error_is_reported_with_its_field_and_layer() {
        let configuration = BTreeMap::from([
            ("name".to_owned(), mapping("user_id")),
            ("nickname".to_owned(), mapping("nick")),
            (
                "balance".to_owned(),
                FieldMapping {
                    external_type: Some("boolean".to_owned()),
                    ..mapping("account_balance")
                },
            ),
        ]);
        let none = BTreeMap::new();
        let layers = Layers {
            configuration: &configuration,
            invocation: &none,
        };
        let header = ["user_id".to_owned()];
        let errors = resolve("users", "csv", &user_row(), layers, Some(&header)).unwrap_err();
        let found: Vec<(ErrorKind, &str, Origin)> = errors
            .iter()
            .map(|error| (error.kind, error.field.as_str(), error.origin))
            .collect();
        assert_eq!(
            found,
            [
                (
                    ErrorKind::UndeclaredField,
                    "nickname",
                    Origin::Configuration
                ),
                (ErrorKind::UnmappedField, "id", Origin::Format),
                (ErrorKind::NoDecoder, "balance", Origin::Configuration),
                (
                    ErrorKind::MissingExternalField,
                    "balance",
                    Origin::Configuration
                ),
            ]
        );
        let configuration = BTreeMap::from([("name".to_owned(), mapping("id"))]);
        let layers = Layers {
            configuration: &configuration,
            invocation: &none,
        };
        let header = ["id".to_owned(), "balance".to_owned()];
        let errors = resolve("users", "csv", &user_row(), layers, Some(&header)).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, ErrorKind::SharedExternalField);
        assert_eq!(errors[0].field, "name");
        assert!(
            errors[0].to_string().contains("read by `id` already"),
            "{}",
            errors[0]
        );
    }

    #[test]
    fn a_lossy_representation_needs_permission() {
        let number = |lossy| {
            BTreeMap::from([(
                "balance".to_owned(),
                FieldMapping {
                    external_type: Some("number".to_owned()),
                    lossy,
                    ..FieldMapping::default()
                },
            )])
        };
        let none = BTreeMap::new();
        let refused = number(None);
        let layers = Layers {
            configuration: &refused,
            invocation: &none,
        };
        let errors = resolve("users", "json", &user_row(), layers, None).unwrap_err();
        assert_eq!(errors[0].kind, ErrorKind::LossyRepresentation);
        assert!(errors[0].message.contains("lossy = true"));
        let permitted = number(Some(true));
        let layers = Layers {
            configuration: &permitted,
            invocation: &none,
        };
        assert!(resolve("users", "json", &user_row(), layers, None).is_ok());
        // Unwritten, a JSON `Decimal` is a string, which keeps its digits.
        let layers = Layers {
            configuration: &none,
            invocation: &none,
        };
        let boundary = resolve("users", "json", &user_row(), layers, None).unwrap();
        assert_eq!(
            boundary.field("balance").unwrap().external_type.value,
            "string"
        );
    }

    #[test]
    fn an_unknown_external_type_has_no_decoder() {
        let configuration = BTreeMap::from([(
            "id".to_owned(),
            FieldMapping {
                external_type: Some("uuid".to_owned()),
                ..FieldMapping::default()
            },
        )]);
        let none = BTreeMap::new();
        let layers = Layers {
            configuration: &configuration,
            invocation: &none,
        };
        let errors = resolve("users", "json", &user_row(), layers, None).unwrap_err();
        assert_eq!(errors[0].kind, ErrorKind::NoDecoder);
        assert!(errors[0].message.contains("no type `uuid`"));
    }
}
