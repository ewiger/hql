// Documents: a header of everything known about one, and a body someone wrote.
//
// See doc/wiki/hql/types/doc-type.hmd and fields.hmd.

// A document body before its dialect is established.
abstract type Content

abstract type Doc {
    header : Data
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
