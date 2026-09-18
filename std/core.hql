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

// compare(a, b) returns Less, Equal, or Greater for one Orderable type.
// Supplying a comparison to sort does not change the element's ancestry.
type Ordering
