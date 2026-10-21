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

// Two values held together, each at its own type: what zip, enumerate,
// entries and join produce, and what a program builds with
// Pair(first = a, second = b).
type Pair<A, B> {
    first  : A
    second : B
}
