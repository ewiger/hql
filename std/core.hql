// The vocabulary the core owns: scalars, the open tree, absence, value order.
// Collections are their own module — see collections.hql.
//
// See doc/wiki/hql/types/data-type.hmd, option-type.hmd, and
// types/collections/collection-types.md.

// Value order: for any two values of the type, which comes first can be
// decided. This is a property of the element, not of any collection holding
// it — a Card is orderable because it has a name, which is what makes a prefix
// of an unordered collection reproducible without discovery order becoming
// observable. Sorting expects the order to be total.
abstract type Orderable

// A leaf: a value with no parts of its own. Being a leaf and having an order
// are separate contracts, which is why Bool satisfies one and not the other.
abstract type Scalar

// Int, Float and String carry the intrinsic orders used by sorting and by
// SortedMap keys. No literal produces a Date yet.
type Bool : Scalar
type Int : {Scalar, Orderable}
type Float : {Scalar, Orderable}
type String : {Scalar, Orderable}
type Date : {Scalar, Orderable}

// An open tree whose keys belong to whoever wrote them: what a YAML or JSON
// document is. Here the alternatives are genuinely alternatives, so this is a
// union rather than a supertype set: a leaf, a sequence of trees, or a
// string-keyed map of trees, and nothing else.
type Data = union {
    Scalar,
    Seq<Data>,
    Map<String, Data>
}

// Absence is an ordinary sum type. There is no null.
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

// A computation that may have failed: ok(value) or err(error). The ways out
// mirror Option's, in the `result` module of the prelude:
// is_ok, is_err, ok() : Option<T>, err() : Option<E>, unwrap_or,
// unwrap_or_else(f: E -> T), map, map_err, and_then, or_else. No unwrap.
type Result<T, E>

// compare(a, b) returns Less, Equal, or Greater for one Orderable type.
// Supplying a comparison to sort does not change the element's ancestry.
type Ordering

// A type written as an argument: `is(Card)`, `of_type(Card)`. A parameter of
// type Type<U> takes a type name and binds U to the type it names.
type Type<T>

// Text parsed by construction: `Data(s, Json)`, `s | Data(Yaml)`. The format
// is one whose Decoder reads Data, Json, JsonLines, Yaml or Csv; a framed
// format, JsonLines or Csv, gives the list of its records, and Json or Yaml
// one document. Text that does not parse in the format fails at run time.
fn Data<F : Format>(text: String, as: Type<F>) : Data

// What a predicate that tests its parameter's type, `x => x | is(U)`, gives:
// a Bool that, where true, holds the parameter at U. A parameter of type
// T -> Is<U> takes only such a predicate, and binds U to the type it tests.
type Is<T>

// The capabilities the language owns, in scope in every program: the
// operators resolve through them, and a declared type implements them for
// its own values. `Self` is the implementing type. A type implementing Ord
// is Orderable: compare, <, sort, min and max take it, and its compare
// decides the order. See doc/proposals/HQL-0024/parts/05-traits.md and
// 06-operators.md.
trait Eq {
    fn eq(self, other: Self) : Bool
}
trait Ord {
    fn compare(self, other: Self) : Ordering
}
trait Add {
    fn add(self, other: Self) : Self
}
trait Sub {
    fn sub(self, other: Self) : Self
}
trait Mul {
    fn mul(self, other: Self) : Self
}
trait Div {
    fn div(self, other: Self) : Self
}
trait Rem {
    fn rem(self, other: Self) : Self
}
// A window of positions, what a slice takes: `xs[a:b]` is
// `xs[Range(start = a, end = b)]`, and a bound left out is absent. A negative
// bound counts from the end. See doc/wiki/hql/operators/bracket-op.hmd.
type Range {
    start? : Int
    end?   : Int
}

// A position: `xs[i]` over a type implementing Index is `xs.at(i)`, the
// element there or its absence. T is the element type, which each
// implementation names: `E : Index<T>` leaves T to the type E is.
trait Index<T> {
    fn at(self, position: Int) : Option<T>
}
// A part: `xs[a:b]` over a type implementing Slice is
// `xs.slice(Range(start = a, end = b))`, a value of the type itself.
trait Slice {
    fn slice(self, range: Range) : Self
}

// The built-in types' implementations of the core traits. A header without a
// body declares a native implementation: the runtime supplies its functions,
// and binding refuses a header with no native body and a native body with no
// header. These are every implementation a built-in type has; a program
// implements a trait only for a type or a trait of its own.
impl Eq for Bool
impl Eq for Int
impl Eq for Float
impl Eq for String
impl Eq for Date
impl Ord for Int
impl Ord for Float
impl Ord for String
impl Ord for Date
impl Add for Int
impl Add for Float
impl Sub for Int
impl Sub for Float
impl Mul for Int
impl Mul for Float
impl Div for Int
impl Div for Float
impl Rem for Int
impl Rem for Float
// Every sequence takes Seq's positions and parts, a List and a Ranking among
// them. A String's positions are its characters, each read as text. Data's
// are a document's own; Data is a union, so a String, one of its
// alternatives, takes its own implementations and not Data's.
impl<T> Index<T> for Seq<T>
impl Index<String> for String
impl Index<Data> for Data
impl<T> Slice for Seq<T>
impl Slice for String
impl Slice for Data

// The operation types, reserved: one per shape a pipeline step may have, and
// their common supertype. Each stands for a function from I to O that also
// carries its shape: what an executor may assume about splitting the step.
// A program declares no type of these names. See
// doc/proposals/HQL-0005/README.md and HQL-0004's shapes.
abstract type Operation<I, O>
abstract type Elementwise<I, O> : Operation<I, O>
abstract type Partition<I, O> : Operation<I, O>
abstract type Reduction<I, O> : Operation<I, O>
abstract type Barrier<I, O> : Operation<I, O>

// Two values held together, each at its own type: what zip, enumerate,
// entries and join produce, and what a program builds with
// Pair(first = a, second = b).
type Pair<A, B> {
    first  : A
    second : B
}
