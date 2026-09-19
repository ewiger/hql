// Documents: a header of everything known about one, and a body someone wrote.
//
// See doc/wiki/hql/types/doc-type.hmd and fields.hmd.

// A document body before its dialect is established.
abstract type Content

// Where a document is and what it is called are facts about the file, so they
// are fields of the document itself. Nothing in the header or the metadata can
// contradict them, because they are not written there.
abstract type Doc {
    name   : String   // its name in its namespace — the file stem for a file
    path   : String   // where it is, relative to the vault root
    format : String   // the dialect its body is written in
    header : Data     // everything the document says about itself
    body   : Content
}

// A dialect narrows the body and may declare a shape over part of the header.
type HmdContent : Content

type HmdHeader {
    metadata? : Data
}

type HmdDoc : Doc {
    header : HmdHeader        // a partial schema: other paths stay open
}
