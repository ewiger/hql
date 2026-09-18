// The vocabulary the core owns: collections, the open tree, absence, order.
//
// See doc/wiki/hql/collections.hmd, data-type.hmd, option-type.hmd.

// An open tree whose keys belong to whoever wrote them.
type Data

// Membership, order and lookup are three different contracts.
type Set<T>         // unordered, distinct elements; no implicit first element
type Seq<T>         // ordered, repeats allowed; position is observable
type Map<K, V>      // lookup by key; iteration order is not implied

// Absence is an ordinary sum type. There is no null.
type Option<T>

// Anything carrying an ordering key of its own, so that a prefix of an
// unordered collection is reproducible without discovery order being
// observable. A Card is orderable because it has a name.
type Orderable
