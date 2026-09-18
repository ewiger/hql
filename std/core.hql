// The vocabulary the core owns: the open tree, absence, value order.
// Collections are their own module — see collections.hql.
//
// See doc/wiki/hql/types/data-type.hmd, option-type.hmd, and
// types/collections/collection-types.md.

// An open tree whose keys belong to whoever wrote them - essentially JSON universe
type Data

// Absence is an ordinary sum type. There is no null.
type Option<T>

// Value order: for any two values of the type, which comes first can be
// decided. This is a property of the element, not of any collection holding
// it — a Card is orderable because it has a name, which is what makes a prefix
// of an unordered collection reproducible without discovery order becoming
// observable. Sorting expects the order to be total.
abstract type Orderable

// Primitive intrinsic orders used by sorting and SortedMap keys.
type Int : Orderable
type Float : Orderable
type String : Orderable

// compare(a, b) returns Less, Equal, or Greater for one Orderable type.
// Supplying a comparison to sort does not change the element's ancestry.
type Ordering
