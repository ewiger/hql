// Module `core`. Implemented by the `std` extension.
//
// See doc/wiki/hql/types/data-type.hmd, option-type.hmd, and
// types/collections/collection-types.md.
/// The types every other module is written in: the scalars, the collection
/// types, the open tree `Data`, absence, failure, value order, the format
/// category and the core traits. A type sits here when a module below it
/// would otherwise import a module above it: `Data` is a tree of sequences
/// and maps, so the collection types are core types, and the operations
/// over them are the `collections` module's.
module core

/// A type whose values have a total order: of any two values of the type,
/// one comes first or the two are equal. `compare`, `<`, `sort`, `min` and
/// `max` take it, and `SortedMap` keys are of it.
/// Order is a property of the element, not of a collection holding it.
abstract type Orderable

/// A leaf: a value with no parts of its own. Being a leaf and having an
/// order are separate contracts: `Bool` is a `Scalar` and not `Orderable`.
abstract type Scalar

/// The truth values `true` and `false`: a `Scalar` with equality and no
/// order.
type Bool : Scalar

/// A 64-bit whole number, ordered by value: `7 / 2` is `3`, and `7 % 2`
/// is `1`.
type Int : {Scalar, Orderable}

/// A 64-bit binary floating-point number, written with the suffix `f`
/// (`2.5f`), ordered by value.
type Float : {Scalar, Orderable}

/// An exact decimal number, written with a decimal point and no suffix
/// (`2.50`), ordered by value: `0.1 + 0.2 == 0.3` is `true`. See
/// [[/learn/02-data-types/decimal|Decimal]].
type Decimal : {Scalar, Orderable}

/// Text, ordered character by character: `"ab" + "cd"` is `"abcd"`, and
/// `"hello"[1]` is `"e"`.
type String : {Scalar, Orderable}

/// A calendar date, ordered by time. No literal produces a `Date`.
type Date : {Scalar, Orderable}

// The collection types. See doc/wiki/hql/types/collections/collection-types.md
// and collection.md.

// See doc/models/behavior/queries.md.
/// A value whose elements can each be visited: the bound an operation
/// states when it only visits each value. Finiteness, order and uniqueness
/// are not part of it.
abstract type Iterable<T>

/// A finite multiset of occurrences, unordered, with no runtime
/// representation. `size(c)` is the number of occurrences, `count(c, x)`
/// the number equal to `x`, and `contains(c, x)` holds where
/// `count(c, x) > 0`; so `size(c)` is the sum of `count(c, x)` over the
/// distinct values in `c`.
abstract type Collection<T> : Iterable<T>

// See doc/wiki/hql/types/stream-type.hmd.
/// An ordered, possibly unending run of values `x0, x1, x2, ...`. Size,
/// membership and global order are undefined on a stream: each requires a
/// last value.
abstract type Stream<T> : Iterable<T>

/// A finite stream: a collection in which every occurrence has exactly one
/// position in `0 .. size(c) - 1`. Positional order does not require
/// `T : Orderable`.
abstract type Seq<T> : {Collection<T>, Stream<T>}

/// A sequence of materialized occurrences; repeated traversal preserves
/// positions. A list literal `[1, 2, 3]` is a `List<Int>`.
type List<T> : Seq<T>

/// Materialized distinct values under HQL equality: `count(s, x) <= 1`.
/// Traversal order is not a semantic guarantee.
type Set<T> : Collection<T>

/// Materialized associations from distinct keys of `K` to values of `V`;
/// not a `Collection`. Its keys are a `Set<K>`, and a lookup gives an
/// `Option<V>`.
type Map<K, V>

/// A `Map` whose keys keep their construction positions: its keys are a
/// `Seq<K>`. `K` need not be `Orderable`. Viewed as a `Map`, its keys are
/// a set.
type OrderedMap<K, V> : Map<K, V>

/// An `OrderedMap` whose key positions follow the intrinsic total order of
/// `K`.
type SortedMap<K, V> : OrderedMap<K, V>
where K : Orderable

// Constructors: a function named after its type builds a value of it, and
// `List(xs)` and `xs | List` are one call, the piped value its first
// argument. A type's forms of one arity differ in their first parameter, so
// the first argument selects one. The binary implements each.
/// The `List` of the elements of `xs`, in order: `List([1, 2])` is
/// `[1, 2]`.
fn List<T>(xs: Seq<T>) : List<T>

/// The `Set` of the distinct elements of `xs`.
fn Set<T>(xs: Collection<T>) : Set<T>

/// The `Map` from each of `keys` to the value at the same position in
/// `values`. The two are equally long, and a repeated key fails at run
/// time.
fn Map<K, V>(keys: Seq<K>, values: Seq<V>) : Map<K, V>

/// The `OrderedMap` from each of `keys` to the value at the same position
/// in `values`, its keys in the order of `keys`.
fn OrderedMap<K, V>(keys: Seq<K>, values: Seq<V>) : OrderedMap<K, V>

/// The `SortedMap` from each of `keys` to the value at the same position
/// in `values`, its keys in the order of `K`.
fn SortedMap<K : Orderable, V>(keys: Seq<K>, values: Seq<V>) : SortedMap<K, V>

/// An open tree whose keys belong to whoever wrote them: what a YAML or
/// JSON document is. It is a union of exactly three alternatives: a
/// `Scalar` leaf, a sequence of trees, or a `String`-keyed map of trees.
type Data = union {
    Scalar,
    Seq<Data>,
    Map<String, Data>
}

// The structural categories (HQL-0030). Whether a structural type is closed
// or open is its supertype's to say, never a modifier's:
//   type A = {…}                 closed: exactly its declared fields
//   type B : Data = {…}          open: at least its fields, a view over Data
//   type C : TypeContract = {…}  closed, and validated as a contract
//   type D : DataContract = {…}  open, and validated as a contract
/// The supertype of a closed contract: a structural type declared
/// `type C : TypeContract = {…}` has exactly its declared fields, and is
/// validated as a contract.
abstract type TypeContract

/// The supertype of an open contract: a structural type declared
/// `type D : DataContract = {…}` is a view over `Data` with at least its
/// declared fields, and is validated as a contract. It is open by being
/// `Data`; no `Data` is written beside it.
abstract type DataContract : Data

/// An optional value of `T`: present, holding one `T`, or absent. Absence is
/// an ordinary sum type; there is no null. The ways out are the `option`
/// module's, and each says what absence means.
type Option<T>

// Resolving absence, always available:
// is_some<T>(o: Option<T>) : Bool
// is_none<T>(o: Option<T>) : Bool
// unwrap_or<T>(o: Option<T>, default: T) : T
// unwrap_or_else<T>(o: Option<T>, f: () -> T) : T
// alternative<T>(o: Option<T>, other: Option<T>) : Option<T>  (`or` is a connective)
// map<T, U>(o: Option<T>, f: T -> U) : Option<U>
// and_then<T, U>(o: Option<T>, f: T -> Option<U>) : Option<U>
// filter<T>(o: Option<T>, p: T -> Bool) : Option<T>
// Over a collection of options:
// somes<F<_> : Iterable<_>, T>(xs: F<Option<T>>) : F<T>   keeps the present values
// There is no unwrap: every way out of an Option says what absence means.

/// A computation that may have failed: `ok(value)` with a `T`, or
/// `err(error)` with an `E`. The ways out are the `result` module's, and
/// mirror `Option`'s; there is no unwrap.
type Result<T, E>

/// The result of `compare(a, b)`: `Less`, `Equal` or `Greater`.
/// `compare(1, 2)` is `Less`.
type Ordering

/// A type written as an argument: `is(Card)`, `of_type(Card)`. A parameter
/// of type `Type<U>` takes a type name and binds `U` to the type it names.
type Type<T>

/// The category of formats: how bytes are documents. Parsing text into
/// `Data`, and every reading and writing operation of `io`, takes a
/// `Format`; the formats and their codecs are the `format` module's.
abstract type Format

/// `text` parsed in the format `as`: `Data("[1, 2]", Json)` is `[1, 2]`,
/// and `s | Data(Yaml)` is one call. The format is one whose decoder reads
/// `Data`: `Json`, `JsonLines`, `Yaml` or `Csv`. A framed format,
/// `JsonLines` or `Csv`, gives the list of its records; `Json` or `Yaml`
/// gives one document. Text that does not parse fails at run time.
fn Data<F : Format>(text: String, as: Type<F>) : Data

/// What a predicate that tests its parameter's type, `x => x | is(U)`,
/// gives: a `Bool` that, where true, holds the parameter at `U`. A
/// parameter of type `T -> Is<U>` takes only such a predicate, and binds
/// `U` to the type it tests.
type Is<T>

// The capabilities the language owns, in scope in every program: the
// operators resolve through them, and a declared type implements them for
// its own values. `Self` is the implementing type. See
// doc/proposals/HQL-0024/parts/05-traits.md and 06-operators.md.
/// Equality: `a == b` and `a != b` over a type implementing `Eq` call its
/// `eq`.
trait Eq {
    fn eq(self, other: Self) : Bool
}

/// Order: `a < b` and the other comparisons over a type implementing `Ord`
/// call its `compare`. A type implementing `Ord` is `Orderable`: `sort`,
/// `min` and `max` take it, and its `compare` gives the order.
trait Ord {
    fn compare(self, other: Self) : Ordering
}

/// Addition: `a + b` over a type implementing `Add` is `a.add(b)`.
trait Add {
    fn add(self, other: Self) : Self
}

/// Subtraction: `a - b` over a type implementing `Sub` is `a.sub(b)`.
trait Sub {
    fn sub(self, other: Self) : Self
}

/// Multiplication: `a * b` over a type implementing `Mul` is `a.mul(b)`.
trait Mul {
    fn mul(self, other: Self) : Self
}

/// Division: `a / b` over a type implementing `Div` is `a.div(b)`.
trait Div {
    fn div(self, other: Self) : Self
}

/// Remainder: `a % b` over a type implementing `Rem` is `a.rem(b)`.
trait Rem {
    fn rem(self, other: Self) : Self
}

// See doc/wiki/hql/operators/bracket-op.hmd.
/// A window of positions, what a slice takes: `xs[a:b]` is
/// `xs[Range(start = a, end = b)]`, and a bound left out is absent. A
/// negative bound counts from the end: `"hello"[-2:]` is `"lo"`.
type Range {
    start? : Int
    end?   : Int
}

/// A position: `xs[i]` over a type implementing `Index` is `xs.at(i)`, the
/// element there or its absence. `T` is the element type, which each
/// implementation names: `E : Index<T>` leaves `T` to the type `E` is.
trait Index<T> {
    fn at(self, position: Int) : Option<T>
}

/// A part: `xs[a:b]` over a type implementing `Slice` is
/// `xs.slice(Range(start = a, end = b))`, a value of the type itself.
trait Slice {
    fn slice(self, range: Range) : Self
}

// Validator types: HQL-0034.
/// A check a value passes: a trait extending `Validator` is a validator
/// type. `validate f with V` holds where `V`'s `validate` of `f` is true,
/// and a field declared `f: V` reads as the type implementing `V`, held to
/// it.
trait Validator {
    fn validate(self) : Bool
}

// The built-in types' implementations of the core traits. A header without a
// body declares a native implementation: the runtime supplies its functions,
// and binding refuses a header with no native body and a native body with no
// header. These are every implementation a built-in type has; a program
// implements a trait only for a type or a trait of its own.
/// `Bool`, `Int`, `Float`, `Decimal`, `String` and `Date` compare with `==`
/// by value.
impl Eq for Bool
impl Eq for Int
impl Eq for Float
impl Eq for Decimal
impl Eq for String
impl Eq for Date

/// `Int`, `Float` and `Decimal` are ordered by value, `String` character by
/// character, and `Date` by time.
impl Ord for Int
impl Ord for Float
impl Ord for Decimal
impl Ord for String
impl Ord for Date

/// `+` adds `Int`s, `Float`s and `Decimal`s, and concatenates two
/// `String`s: `"ab" + "cd"` is `"abcd"`.
impl Add for Int
impl Add for Float
impl Add for Decimal
impl Add for String

/// `-` subtracts `Int`s, `Float`s and `Decimal`s.
impl Sub for Int
impl Sub for Float
impl Sub for Decimal

/// `*` multiplies `Int`s, `Float`s and `Decimal`s.
impl Mul for Int
impl Mul for Float
impl Mul for Decimal

/// `/` divides `Int`s, `Float`s and `Decimal`s: `7 / 2` is `3`.
impl Div for Int
impl Div for Float
impl Div for Decimal

/// `%` is the remainder of `Int`s, `Float`s and `Decimal`s: `7 % 2` is `1`.
impl Rem for Int
impl Rem for Float
impl Rem for Decimal

// Data is a union, so a String, one of its alternatives, takes its own
// implementations and not Data's.
/// `xs[i]` over every sequence, a `List` among them, is the element at
/// `i`; over a `String` it is the character there, as text (`"hello"[1]`
/// is `"e"`); over `Data` it is the document's own element.
impl<T> Index<T> for Seq<T>
impl Index<String> for String
impl Index<Data> for Data

/// `xs[a:b]` over every sequence, a `String` and `Data` is the part of the
/// same type: `[1, 2, 3][1:]` is `[2, 3]`, and `"hello"[1:3]` is `"el"`.
impl<T> Slice for Seq<T>
impl Slice for String
impl Slice for Data

// The operation types, reserved: one per shape a pipeline step may have, and
// their common supertype. A program declares no type of these names. See
// doc/proposals/HQL-0005/README.md and HQL-0004's shapes.
/// A function from `I` to `O` that also carries its shape: what an executor
/// may assume about splitting the step. A value of one is called as an
/// `I -> O` function is, and a lambda bound at one is an operation on equal
/// terms with `map` or `filter`.
abstract type Operation<I, O>

/// An `Operation` whose result for each element depends on that element
/// alone, so it may run over any split of its input: `map` and `filter`
/// are elementwise. Piped a collection of `I`, it runs over each element,
/// and its body reads its one element and nothing else:
/// `long : Elementwise<Int, Bool> = n => n > 300`.
abstract type Elementwise<I, O> : Operation<I, O>

/// An `Operation` that regroups its input by a key, each group running
/// independently and completing in any order: `group` is one.
abstract type Partition<I, O> : Operation<I, O>

/// An `Operation` that collapses its whole input to one value through a
/// combiner the runtime holds, so each split may be reduced and the results
/// combined: `size` combines by addition, `contains` by disjunction.
abstract type Reduction<I, O> : Operation<I, O>

/// An `Operation` that needs its whole input before any output, so it is
/// not split: `sort`, `take` and `reduce` are barriers.
abstract type Barrier<I, O> : Operation<I, O>

/// Two values held together, each at its own type: what `zip`,
/// `enumerate`, `entries` and `join` produce, and what
/// `Pair(first = a, second = b)` builds.
type Pair<A, B> {
    first  : A
    second : B
}

// What `validate(T)` reports (HQL-0033).
/// One reason a value does not read as a contract, as `validate(T)`
/// reports it. `kind` is `"missing"`, `"undeclared"`, `"type"` or
/// `"rule"`; `rule` is the rule's identity, `Contract.name`, or
/// `Contract.n` for the n-th unnamed rule; `path` leads from the value to
/// the field, `[]` for the value itself.
type Violation : DataContract {
    contract  : String
    path      : List<String>
    kind      : String
    rule      : String
    text      : String
    expected? : String
    actual?   : Data
    severity  : String = "error"
    message   : String
}

/// A value that did not read as a contract, and every reason why: its
/// `contract`, the `value`, and the `violations`.
type Invalid : DataContract {
    contract   : String
    value      : Data
    violations : List<Violation>
}
