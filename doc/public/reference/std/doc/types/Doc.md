<!-- hql reference: std/doc.Doc -->
# `Doc`

In [`std/doc`](../index.md).

```hql
abstract type Doc : Orderable { name : String // its name in its namespace — the file stem for a file title : String // authored, else the first heading, else the name path : String // where it is, relative to the vault root format : String // the dialect its body is written in header : Data // everything the document says about itself body : String // the text after the header, as written }
```

A document: its `name`, `title`, `path` and `format`, its `header` and
its `body`. Where a document is and what it is called are facts about
the file, so they are fields of the document itself, and nothing in the
header or the metadata can contradict them. A document is ordered by
its `name`, so every `Card` is `Orderable` without declaring it.
<!-- hql reference: end -->
