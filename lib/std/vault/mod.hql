// The current vault: the source a program runs against.
//
// See doc/issues/0010-yield-and-bind.md and doc/models/behavior/queries.md.
//
// A host with a vault seeds it as `vault` into the program module; a program
// running without one has no `vault` binding, and no module sees it.
module vault
import core
import doc
import graph

type Vault {
    name : String    // the vault's directory name
    root : String    // where the vault is
    graph : Graph    // every card as a node, and the links among them
    // Every card, in the order a sorted walk reaches it: the entries of each
    // directory sorted by name, depth first. The order is the walk's, never
    // the filesystem's, so `take(5)` answers the same on every machine.
    cards : Seq<Card>
}
