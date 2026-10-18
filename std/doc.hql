// Documents: a header of everything known about one, and a body someone wrote.
//
// See doc/wiki/hql/types/doc-type.hmd and fields.hmd.

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
