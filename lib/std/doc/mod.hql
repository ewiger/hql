// Module `doc`: documents, a header of everything known about one and a body
// someone wrote, and the card, a document a vault holds. Every declaration is
// a type the runtime constructs; the module declares no operation.
//
// See doc/wiki/hql/types/doc-type.hmd and fields.hmd.
module doc
import core

// Where a document is and what it is called are facts about the file, so they
// are fields of the document itself. Nothing in the header or the metadata can
// contradict them, because they are not written there. A document is ordered by
// its name, so every Card is Orderable without declaring it.
abstract type Doc : Orderable {
    name   : String   // its name in its namespace — the file stem for a file
    title  : String   // authored, else the first heading, else the name
    path   : String   // where it is, relative to the vault root
    format : String   // the dialect its body is written in
    header : Data     // everything the document says about itself
    body   : String   // the text after the header, as written
}

// A dialect may declare a shape over part of the header.
type HmdHeader {
    metadata? : Data
}

type HmdDoc : Doc {
    header : HmdHeader        // a partial schema: other paths stay open
}

// Card is the base representation type: knowledge given a physical document.
// It sits with the document rather than with knowledge because a graph's
// nodes are cards and knowledge narrows the graph: graph and knowledge both
// import it. Its two kinds, which `knowledge` declares, say what the document
// is about.
abstract type Card : Doc {
    metadata : Data     // the knowledge attached to this card, assembled
    kind     : String   // "ConceptCard" or "RelationCard"
}

// Card is abstract in the other direction too: a concrete card is one whose
// body is written in a given dialect, one subtype per `header.format`.
type HmdCard : {Card, HmdDoc} // format is hmd
type MdCard  : Card           // format is md, which no dialect type narrows
