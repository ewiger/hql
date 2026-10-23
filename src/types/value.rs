//! Runtime values: what an evaluated expression is, what type it has, and
//! the ordering key it carries of its own.
//!
//! How a value prints is [`display`](super::display).

use super::{MapKind, MapValue, TypeRef};
use crate::data::Data;
use crate::decimal::Decimal;
use crate::document::{Card, Document, Kind};
use crate::graph::{Edge, Graph};
use crate::search::{Hit, Ranking};
use std::collections::BTreeMap;
use std::sync::Arc;

/// How the text leaves of a tree read when it is decoded as a declared type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leaves {
    /// A text leaf is a `String`, whatever it spells.
    AsWritten,
    /// A text leaf is read as the scalar its place wants: a CSV cell.
    Text,
}

/// Why a value does not decode as a type: the dotted path of the field that
/// does not, empty for the value itself, and what it holds instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    /// The field's path from the value decoded, outermost first.
    pub path: Vec<String>,
    /// What the field holds, or that it is missing.
    pub reason: String,
}

impl Mismatch {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            path: Vec::new(),
            reason: reason.into(),
        }
    }

    /// The mismatch seen from the value that holds it at `field`.
    fn within(mut self, field: &str) -> Self {
        self.path.insert(0, field.to_owned());
        self
    }
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.reason)
        } else {
            write!(f, "field `{}`: {}", self.path.join("."), self.reason)
        }
    }
}

/// Every reason a value does not read as a type, each field's in the order
/// the type declares it: reading reports them all, not only the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatches(pub Vec<Mismatch>);

impl Mismatches {
    /// Each mismatch seen from the value that holds it at `field`.
    fn within(self, field: &str) -> Self {
        Self(
            self.0
                .into_iter()
                .map(|mismatch| mismatch.within(field))
                .collect(),
        )
    }
}

impl From<Mismatch> for Mismatches {
    fn from(mismatch: Mismatch) -> Self {
        Self(vec![mismatch])
    }
}

impl std::fmt::Display for Mismatches {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let each: Vec<String> = self.0.iter().map(ToString::to_string).collect();
        f.write_str(&each.join("; "))
    }
}

/// A text leaf read as the scalar `wanted` names, or as the text it is.
fn parsed_leaf(text: &str, wanted: &TypeRef) -> Result<Value, Mismatch> {
    let refused = |name: &str| Mismatch::new(format!("{text:?} is not {name}"));
    if *wanted == TypeRef::INT {
        return text
            .trim()
            .parse()
            .map(Value::Int)
            .map_err(|_| refused("an Int"));
    }
    if *wanted == TypeRef::FLOAT {
        return text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(Value::Float)
            .ok_or_else(|| refused("a Float"));
    }
    if *wanted == TypeRef::DECIMAL {
        return Decimal::parse(text)
            .map(Value::Decimal)
            .ok_or_else(|| refused("a Decimal"));
    }
    if *wanted == TypeRef::BOOL {
        return match text.trim() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(refused("a Bool")),
        };
    }
    Ok(Value::Str(Arc::from(text)))
}

/// A result of evaluating an HQL expression.
#[derive(Debug, Clone)]
pub enum Value {
    /// A declaration's result: nothing to show.
    Unit,
    /// The result of a total comparison.
    Ordering(std::cmp::Ordering),
    /// A finite map with checked unique keys.
    Map(Arc<MapValue>),
    /// A signed 64-bit integer.
    Int(i64),
    /// A finite double-precision number.
    Float(f64),
    /// An exact decimal number with its scale.
    Decimal(Decimal),
    /// A Boolean truth value.
    Bool(bool),
    /// Text.
    Str(Arc<str>),
    /// Absence, carrying the type of what is not there.
    Absent(TypeRef),
    /// Presence retains the optional type instead of erasing its wrapper.
    Present(Arc<Value>),
    /// An open tree.
    Data(Arc<Data>),
    /// A document.
    Doc(Arc<Document>),
    /// A card.
    Card(Arc<Card>),
    /// A directed connection.
    Edge(Arc<Edge>),
    /// A graph.
    Graph(Arc<Graph>),
    /// Rows under ordered columns.
    Table(Arc<Table>),
    /// An unordered collection, with the type of its elements.
    Set(Arc<Vec<Value>>, TypeRef),
    /// An ordered collection, with the type of its elements.
    List(Arc<Vec<Value>>, TypeRef),
    /// One scored result.
    Hit(Arc<Hit>),
    /// An ordered collection of hits.
    Ranking(Arc<Ranking>),
    /// One partition of a `group`.
    Group(Arc<Group>),
    /// A value of a declared record type: its fields, in declaration order.
    Record(Arc<Record>),
    /// A computation that succeeded, with the type of the error it did not
    /// produce.
    Ok(Arc<Value>, TypeRef),
    /// A computation that failed, with the type of the value it did not
    /// produce.
    Err(Arc<Value>, TypeRef),
    /// A function: parameters, a body, and the names it captured.
    Function(Arc<Closure>),
    /// Text in a presentation format, such as `Markdown` or `Json`.
    Formatted(Arc<Formatted>),
    /// A vault, whose fields are what a program reads from it: the one the
    /// program runs against, or a configured vault source, `source("notes")`.
    Vault(Option<Arc<crate::vault::Vault>>),
    /// A generated sequence or stream, produced on demand.
    Traversal(Arc<crate::traversal::Traversal>),
    /// Where values come from or go to: `stdin`, `file("x.json")`.
    Endpoint(Arc<crate::io::Endpoint>),
    /// How severe a log event is: a value of `Info`, or another level.
    Level(crate::extensions::logging::Level),
    /// One structured log event.
    Log(Arc<crate::extensions::logging::Event>),
    /// A producer of log events, and where it writes them.
    Logger(Arc<crate::extensions::logging::Logger>),
    /// The cell `hql render` is running.
    Cell,
}

/// Text in one format: a `String` value that also carries the format it was
/// encoded in, for presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formatted {
    /// The text.
    pub text: String,
    /// The format it was encoded in.
    pub format: super::TypeConstructor,
}

/// A function value: what it takes, what it evaluates, and the bindings in
/// scope where it was written, so its free names keep their values.
#[derive(Debug)]
pub struct Closure {
    /// The name a declared function calls itself by.
    pub(crate) name: Option<String>,
    pub(crate) parameters: Vec<String>,
    pub(crate) body: Arc<crate::execution::Expr>,
    pub(crate) captured: std::collections::HashMap<String, Value>,
    pub(crate) ty: TypeRef,
}

/// A value of a declared record type.
#[derive(Debug, Clone)]
pub struct Record {
    /// The record's type, with its arguments as checked at construction.
    pub ty: TypeRef,
    /// Each field's value, in declaration order. An optional field holds an
    /// `Option` value: present, or absent at the field's type.
    pub fields: Vec<(String, Value)>,
    /// The whole tree an open record was read from, undeclared entries
    /// included: reading it as another type sees all of it (HQL-0030). A
    /// closed record, or one built from its fields, has none.
    pub carrier: Option<Arc<Data>>,
}

impl Record {
    /// The value of a field, when the record has one of that name.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, value)| value)
    }
}

/// Rows under ordered columns: `Table(xs, columns = [...])`. Each row is a
/// record or a `Data` map, and a cell is the row's field of its column's
/// name.
#[derive(Debug, Clone)]
pub struct Table {
    /// The column names, in the order they are written.
    pub columns: Vec<String>,
    /// The rows, each a record or a `Data` map, in input order.
    pub rows: Vec<Value>,
}

impl Table {
    /// A table over `rows`: the columns named, or else the first row's
    /// fields in the order it holds them.
    ///
    /// # Errors
    /// Names a row that is neither a record nor a `Data` map.
    pub fn new(rows: Vec<Value>, columns: Option<Vec<String>>) -> Result<Self, String> {
        let rows: Vec<Value> = rows
            .into_iter()
            .map(|row| match row {
                Value::Present(held) => held.as_ref().clone(),
                other => other,
            })
            .collect();
        if let Some(row) = rows.iter().find(|row| Self::fields(row).is_none()) {
            return Err(format!(
                "a Table row is a record or a Data map, not {}",
                row.type_of()
            ));
        }
        let columns = match columns {
            Some(columns) => columns,
            None => rows.first().and_then(Self::fields).unwrap_or_default(),
        };
        Ok(Self { columns, rows })
    }

    /// The field names of a row, in the order it holds them.
    fn fields(row: &Value) -> Option<Vec<String>> {
        match row {
            Value::Record(record) => {
                Some(record.fields.iter().map(|(name, _)| name.clone()).collect())
            }
            Value::Data(data) => match data.as_ref() {
                Data::Map(entries) => Some(entries.keys().cloned().collect()),
                _ => None,
            },
            _ => None,
        }
    }

    /// The cell of `row` under `column`: the row's field of that name, or
    /// `None` when it has none or holds an absent optional.
    #[must_use]
    pub fn cell(row: &Value, column: &str) -> Option<Value> {
        let found = match row {
            Value::Record(record) => record.field(column).cloned(),
            Value::Data(data) => match data.as_ref() {
                Data::Map(entries) => entries
                    .get(column)
                    .map(|held| Value::Data(Arc::new(held.clone()))),
                _ => None,
            },
            _ => None,
        }?;
        match found {
            Value::Absent(_) => None,
            Value::Present(held) => Some(held.as_ref().clone()),
            other => Some(other),
        }
    }
}

/// One partition of a `group`: a key and the occurrences that have it.
#[derive(Debug, Clone)]
pub struct Group {
    /// The value the key selector returned.
    pub key: Value,
    /// The occurrences with that key, in input order.
    pub items: Vec<Value>,
    /// The key's checked type.
    pub key_type: TypeRef,
    /// The occurrences' checked type.
    pub item_type: TypeRef,
}

impl Value {
    /// Expose a checked annotation's map contract without changing storage.
    pub(crate) fn viewed_as(self, expected: &TypeRef) -> Self {
        match (&self, MapKind::of(expected.constructor)) {
            (Self::Map(map), Some(_)) => Self::Map(Arc::new(map.viewed_as(expected))),
            (Self::Present(value), _) if expected.constructor.0 == "Option" => Self::Present(
                Arc::new(value.as_ref().clone().viewed_as(expected.present())),
            ),
            (Self::List(values, _), _) if expected.is_collection() => {
                let Some(element) = expected.element() else {
                    return self;
                };
                Self::list(
                    values
                        .iter()
                        .cloned()
                        .map(|value| value.viewed_as(&element))
                        .collect(),
                    element,
                )
            }
            (Self::Set(values, _), _) if expected.is_collection() => {
                let Some(element) = expected.element() else {
                    return self;
                };
                Self::set(
                    values
                        .iter()
                        .cloned()
                        .map(|value| value.viewed_as(&element))
                        .collect(),
                    element,
                )
            }
            _ => self,
        }
    }

    /// A set of values of a known element type.
    #[must_use]
    pub fn set(values: Vec<Self>, element: TypeRef) -> Self {
        let mut unique = Vec::with_capacity(values.len());
        for value in values {
            if !unique.contains(&value) {
                unique.push(value);
            }
        }
        Self::Set(Arc::new(unique), element)
    }

    /// A sequence of values of a known element type.
    #[must_use]
    pub fn list(values: Vec<Self>, element: TypeRef) -> Self {
        Self::List(Arc::new(values), element)
    }

    /// The type of the value.
    #[must_use]
    pub fn type_of(&self) -> TypeRef {
        match self {
            Self::Traversal(traversal) => traversal.type_of(),
            Self::Vault(_) => TypeRef::VAULT,
            Self::Endpoint(endpoint) => endpoint.type_of(),
            Self::Level(level) => level.type_of(),
            Self::Log(_) => TypeRef::LOG_EVENT,
            Self::Logger(_) => TypeRef::LOGGER,
            Self::Cell => TypeRef::CELL,
            Self::Unit => TypeRef::UNIT,
            Self::Ordering(_) => TypeRef::ORDERING,
            Self::Map(map) => map.type_of(),
            Self::Int(_) => TypeRef::INT,
            Self::Float(_) => TypeRef::FLOAT,
            Self::Decimal(_) => TypeRef::DECIMAL,
            Self::Bool(_) => TypeRef::BOOL,
            Self::Str(_) => TypeRef::STR,
            Self::Absent(element) => TypeRef::optional(element.clone()),
            Self::Present(value) => TypeRef::optional(value.type_of()),
            Self::Data(_) => TypeRef::DATA,
            Self::Doc(_) => TypeRef::DOC,
            Self::Card(card) => match card.kind {
                Kind::Concept => TypeRef::CONCEPT_CARD,
                Kind::Relation => TypeRef::RELATION_CARD,
            },
            Self::Edge(_) => TypeRef::EDGE,
            Self::Graph(_) => TypeRef::GRAPH,
            Self::Table(_) => TypeRef::TABLE,
            Self::Set(_, element) => TypeRef::set(element.clone()),
            Self::List(_, element) => TypeRef::list(element.clone()),
            Self::Hit(hit) => TypeRef::hit(hit.element.clone()),
            Self::Ranking(ranking) => TypeRef::ranking(ranking.element.clone()),
            Self::Function(function) => function.ty.clone(),
            // Encoded text is a String; the format it is in rides along for a
            // host to print it as it is.
            Self::Formatted(_) => TypeRef::STR,
            Self::Group(group) => {
                super::builtin::GROUP.apply([group.key_type.clone(), group.item_type.clone()])
            }
            Self::Record(record) => record.ty.clone(),
            Self::Ok(value, error) => TypeRef::result(value.type_of(), error.clone()),
            Self::Err(error, value) => TypeRef::result(value.clone(), error.type_of()),
        }
    }

    /// The elements of any collection, in the order they are held.
    #[must_use]
    pub fn elements(&self) -> Option<Vec<Self>> {
        match self {
            Self::Set(values, _) | Self::List(values, _) => Some(values.as_ref().clone()),
            Self::Traversal(traversal) => traversal.forced().map(<[Self]>::to_vec),
            Self::Ranking(ranking) => Some(
                ranking
                    .hits
                    .iter()
                    .map(|hit| Self::Hit(Arc::new(hit.clone())))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// A traversal that has been forced, as the list of what it produced, so
    /// it renders and compares as the values it holds.
    pub(crate) fn settled(&self) -> Option<Self> {
        match self {
            Self::Traversal(traversal) => traversal
                .forced()
                .map(|values| Self::list(values.to_vec(), traversal.element.clone())),
            _ => None,
        }
    }

    /// The card a value is, or the card a hit retrieved.
    #[must_use]
    pub fn as_card(&self) -> Option<Arc<Card>> {
        match self {
            Self::Card(card) => Some(Arc::clone(card)),
            Self::Present(value) => value.as_card(),
            Self::Hit(hit) => hit.card().cloned(),
            _ => None,
        }
    }

    /// The value's own ordering key, or `None` when it has no order.
    ///
    /// This is what makes a prefix of an unordered collection reproducible:
    /// the order is a property of the values, never of how they were found.
    #[must_use]
    pub fn order_key(&self) -> Option<Key> {
        match self {
            Self::Int(value) => Some(Key::Integer(*value)),
            Self::Float(value) => Some(Key::Number(if *value == 0.0 { 0.0 } else { *value })),
            Self::Decimal(value) => Some(Key::Exact(value.clone())),
            Self::Str(text) => Some(Key::text(text)),
            Self::Formatted(formatted) => Some(Key::text(&formatted.text)),
            Self::Doc(doc) => Some(Key::text(&doc.name)),
            Self::Card(card) => Some(Key::text(card.name())),
            // Best first, then the card's own key, so equal scores are stable.
            Self::Hit(hit) => Some(Key::Scored(-hit.score, hit.name())),
            _ => None,
        }
    }

    /// The value read as `wanted`, when it is one.
    ///
    /// A declared value is a `wanted` when it was built as one or as a
    /// subtype: the nominal test. A tree is tested by its shape instead, and
    /// a leaf is read as the scalar it holds, so the integer leaf of a header
    /// is an `Int` and comes back as one.
    #[must_use]
    pub fn interpreted(&self, wanted: &TypeRef) -> Option<Self> {
        self.decoded(wanted, Leaves::AsWritten).ok()
    }

    /// The value read as `wanted`, as [`Value::interpreted`] reads it, or
    /// what keeps it from being one: the dotted path of the field that does
    /// not read, empty for the value itself, and why. With
    /// [`Leaves::Text`], a text leaf wanted as an `Int`, a `Float` or a
    /// `Bool` is parsed as one, and an empty one wanted at an optional field
    /// is absent: a CSV cell is text whatever it holds.
    ///
    /// # Errors
    ///
    /// The value, or a field of it, is not a `wanted`.
    pub fn decoded(&self, wanted: &TypeRef, leaves: Leaves) -> Result<Self, Mismatches> {
        if self.type_of().is(wanted) {
            return Ok(self.clone());
        }
        // An open record is a view: read as another type, it is the tree it
        // was read from that is read.
        if let Self::Record(record) = self
            && let Some(carrier) = &record.carrier
        {
            return Self::Data(Arc::clone(carrier)).decoded(wanted, leaves);
        }
        let Self::Data(tree) = self else {
            return Err(Mismatch::new(format!("{} is not {wanted}", self.type_of())).into());
        };
        let read = match tree.as_ref() {
            Data::Bool(value) => Self::Bool(*value),
            // A number leaf wanted as a Decimal keeps the digits it holds;
            // an integer one widens (HQL-0031).
            Data::Int(value) if *wanted == TypeRef::DECIMAL => {
                Self::Decimal(Decimal::from_int(*value))
            }
            Data::Int(value) => Self::Int(*value),
            Data::Float(value) if *wanted == TypeRef::DECIMAL => Decimal::from_float(*value)
                .map(Self::Decimal)
                .ok_or_else(|| Mismatch::new(format!("{value} is not a Decimal")))?,
            Data::Float(value) => Self::Float(*value),
            // Read as anything else, a decimal leaf is the Float it was
            // before Decimal existed, so an untyped tree reads as it did.
            Data::Decimal(value) if !TypeRef::FLOAT.is(wanted) && TypeRef::DECIMAL.is(wanted) => {
                Self::Decimal(value.clone())
            }
            Data::Decimal(value) => value
                .to_float()
                .map(Self::Float)
                .ok_or_else(|| Mismatch::new(format!("{value} is not a Float")))?,
            Data::Str(text) => match leaves {
                Leaves::Text => parsed_leaf(text, wanted)?,
                Leaves::AsWritten => Self::Str(Arc::from(text.as_str())),
            },
            Data::List(items) => {
                let element = wanted
                    .element()
                    .ok_or_else(|| Mismatch::new(format!("a list is not {wanted}")))?;
                let mut read = Vec::with_capacity(items.len());
                let mut failed = Vec::new();
                for (index, item) in items.iter().enumerate() {
                    match Self::Data(Arc::new(item.clone())).decoded(&element, leaves) {
                        Ok(value) => read.push(value),
                        Err(mismatches) => failed.extend(mismatches.within(&index.to_string()).0),
                    }
                }
                if !failed.is_empty() {
                    return Err(Mismatches(failed));
                }
                let list = Self::list(read, element);
                return if list.type_of().is(wanted) {
                    Ok(list)
                } else {
                    Err(Mismatch::new(format!("a list is not {wanted}")).into())
                };
            }
            Data::Map(entries) => return Self::record_from(entries, wanted, leaves),
            Data::Empty => return Err(Mismatch::new(format!("nothing is not {wanted}")).into()),
        };
        if read.type_of().is(wanted) {
            Ok(read)
        } else {
            Err(Mismatch::new(format!("{} is not {wanted}", read.to_literal())).into())
        }
    }

    /// A tree read as a declared record: each field's entry read at the
    /// field's type, and an optional field without an entry absent. A
    /// closed record (HQL-0030) is exactly its fields, so an undeclared
    /// entry is a mismatch; an open one, a record narrowing `Data`, ignores
    /// it and keeps the whole tree beneath the view. Every field that does
    /// not read is reported.
    fn record_from(
        entries: &BTreeMap<String, Data>,
        wanted: &TypeRef,
        leaves: Leaves,
    ) -> Result<Self, Mismatches> {
        let shape = super::program::tree_shape(wanted)
            .ok_or_else(|| Mismatch::new(format!("a map is not {wanted}")))?;
        let open = wanted.is(&TypeRef::DATA);
        let mut fields = Vec::with_capacity(shape.fields.len());
        let mut failed = Vec::new();
        for field in &shape.fields {
            let entry = entries.get(&field.name).filter(|entry| match entry {
                Data::Empty => false,
                Data::Str(text) => {
                    !(text.is_empty()
                        && field.optional
                        && leaves == Leaves::Text
                        && field.ty != TypeRef::STR)
                }
                _ => true,
            });
            let value = match (entry, field.optional) {
                (Some(entry), optional) => {
                    match Self::Data(Arc::new(entry.clone())).decoded(&field.ty, leaves) {
                        Ok(read) if optional => Self::Present(Arc::new(read)),
                        Ok(read) => read,
                        Err(mismatches) => {
                            failed.extend(mismatches.within(&field.name).0);
                            continue;
                        }
                    }
                }
                (None, true) => Self::Absent(field.ty.clone()),
                // A contract's default stands in for the entry; the tree a
                // view keeps is still the one that was read.
                (None, false) if let Some(default) = &field.default => {
                    match Self::Data(Arc::new(default.clone())).decoded(&field.ty, leaves) {
                        Ok(read) => read,
                        Err(mismatches) => {
                            failed.extend(mismatches.within(&field.name).0);
                            continue;
                        }
                    }
                }
                (None, false) => {
                    failed.push(Mismatch::new("it is missing").within(&field.name));
                    continue;
                }
            };
            fields.push((field.name.clone(), value));
        }
        if !open {
            for key in entries.keys() {
                if !shape.fields.iter().any(|field| field.name == *key) {
                    failed.push(
                        Mismatch::new(format!("{wanted} declares no such field")).within(key),
                    );
                }
            }
        }
        if !failed.is_empty() {
            return Err(Mismatches(failed));
        }
        Ok(Self::Record(Arc::new(Record {
            ty: wanted.clone(),
            fields,
            carrier: open.then(|| Arc::new(Data::Map(entries.clone()))),
        })))
    }

    /// Whether the value counts as true where a condition is expected.
    #[must_use]
    pub fn truth(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

/// A value's intrinsic order, retaining exact integer precision.
#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    /// An exact signed integer.
    Integer(i64),
    /// A finite floating-point number.
    Number(f64),
    /// An exact decimal number, which an integer key compares with by
    /// value.
    Exact(Decimal),
    /// A text key.
    Text(String),
    /// A retrieval score with a deterministic document tiebreak.
    Scored(f64, String),
}

impl Key {
    fn text(value: &str) -> Self {
        Self::Text(value.to_owned())
    }

    /// Compare compatible keys, with deterministic ordering across key families.
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a.cmp(b),
            (Self::Number(a), Self::Number(b)) => a.total_cmp(b),
            (Self::Exact(a), Self::Exact(b)) => a.cmp(b),
            // An Int widens to a Decimal (HQL-0031).
            (Self::Integer(a), Self::Exact(b)) => Decimal::from_int(*a).cmp(b),
            (Self::Exact(a), Self::Integer(b)) => a.cmp(&Decimal::from_int(*b)),
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
            (Self::Scored(a, x), Self::Scored(b, y)) => a.total_cmp(b).then_with(|| x.cmp(y)),
            _ => self.family().cmp(&other.family()),
        }
    }

    fn family(&self) -> u8 {
        match self {
            Self::Integer(_) => 0,
            Self::Number(_) => 1,
            Self::Exact(_) => 2,
            Self::Text(_) => 3,
            Self::Scored(..) => 4,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if let Some(settled) = self.settled() {
            return settled == *other;
        }
        if let Some(settled) = other.settled() {
            return *self == settled;
        }
        match (self, other) {
            (Self::Traversal(a), Self::Traversal(b)) => Arc::ptr_eq(a, b),
            (Self::Vault(a), Self::Vault(b)) => match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            },
            (Self::Unit, Self::Unit) => true,
            (Self::Ordering(a), Self::Ordering(b)) => a == b,
            (Self::Map(a), Self::Map(b)) => a == b,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            // By value, so `1.0 == 1.00`, and an Int widens (HQL-0031).
            (Self::Decimal(left), Self::Decimal(right)) => left == right,
            (Self::Decimal(left), Self::Int(right)) | (Self::Int(right), Self::Decimal(left)) => {
                *left == Decimal::from_int(*right)
            }
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Str(left), Self::Str(right)) => left == right,
            // Formatted text is a String, so it equals a String of the same text.
            (Self::Formatted(left), Self::Formatted(right)) => left == right,
            (Self::Formatted(formatted), Self::Str(text))
            | (Self::Str(text), Self::Formatted(formatted)) => formatted.text == text.as_ref(),
            (Self::Absent(left), Self::Absent(right)) => left == right,
            (Self::Present(left), Self::Present(right)) => left == right,
            (Self::Data(left), Self::Data(right)) => left == right,
            // Heavier values compare by what identifies them, not structurally.
            (Self::Doc(left), Self::Doc(right)) => left.path == right.path,
            (Self::Card(left), Self::Card(right)) => left.document.path == right.document.path,
            (Self::Edge(left), Self::Edge(right)) => {
                left.source == right.source
                    && left.target == right.target
                    && left.data == right.data
                    && left.kind == right.kind
            }
            (Self::Set(left, _), Self::Set(right, _)) => {
                left.len() == right.len() && left.iter().all(|value| right.contains(value))
            }
            (Self::List(left, _), Self::List(right, _)) => left == right,
            (Self::Graph(a), Self::Graph(b)) => Arc::ptr_eq(a, b),
            (Self::Table(a), Self::Table(b)) => a.columns == b.columns && a.rows == b.rows,
            (Self::Hit(a), Self::Hit(b)) => {
                Arc::ptr_eq(a, b)
                    || (a.value == b.value
                        && a.score == b.score
                        && self.to_json() == other.to_json())
            }
            (Self::Ranking(a), Self::Ranking(b)) => {
                Arc::ptr_eq(a, b) || self.to_json() == other.to_json()
            }
            // Two groups are equal iff their keys are.
            (Self::Group(left), Self::Group(right)) => left.key == right.key,
            // Two records are equal when they are of one type and agree
            // field by field.
            (Self::Record(left), Self::Record(right)) => {
                left.ty.constructor == right.ty.constructor && left.fields == right.fields
            }
            (Self::Ok(left, _), Self::Ok(right, _)) | (Self::Err(left, _), Self::Err(right, _)) => {
                left == right
            }
            // A function is equal only to itself: equal behaviour is undecidable.
            (Self::Function(left), Self::Function(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }
}
