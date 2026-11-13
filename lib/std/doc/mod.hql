// The runtime constructs every type here.
//
// See doc/wiki/hql/types/doc-type.hmd and fields.hmd.

/// Documents: a header of everything known about one and a body someone
/// wrote, and the card, a document a vault holds. Every declaration is a
/// type; the module declares no operation.
module doc

import core

/// A document: its `name`, `title`, `path` and `format`, its `header` and
/// its `body`. Where a document is and what it is called are facts about
/// the file, so they are fields of the document itself, and nothing in the
/// header or the metadata can contradict them. A document is ordered by
/// its `name`, so every `Card` is `Orderable` without declaring it.
abstract type Doc : Orderable {
    name   : String   // its name in its namespace — the file stem for a file
    title  : String   // authored, else the first heading, else the name
    path   : String   // where it is, relative to the vault root
    format : String   // the dialect its body is written in
    header : Data     // everything the document says about itself
    body   : String   // the text after the header, as written
}

/// The shape a hyper-markdown header declares over part of itself: an
/// optional `metadata` tree.
type HmdHeader {
    metadata? : Data
}

/// A document written in hyper-markdown: a `Doc` whose `header` is an
/// `HmdHeader`, a partial schema in which other paths stay open.
type HmdDoc : Doc {
    header : HmdHeader        // a partial schema: other paths stay open
}

// Card sits with the document rather than with knowledge because a graph's
// nodes are cards and knowledge narrows the graph: graph and knowledge both
// import it.

/// A card: knowledge given a physical document, the document a vault
/// holds. Its `metadata` is the knowledge attached to it, assembled, and
/// its `kind` is `"ConceptCard"` or `"RelationCard"`, the two kinds module
/// `knowledge` declares, which say what the document is about.
abstract type Card : Doc {
    metadata : Data     // the knowledge attached to this card, assembled
    kind     : String   // "ConceptCard" or "RelationCard"
}

/// A card whose body is written in hyper-markdown, `format` `Hmd`.
type HmdCard : {Card, HmdDoc} // format is hmd

/// A card whose body is written in Markdown, which no dialect type narrows.
type MdCard  : Card           // format is md, which no dialect type narrows
