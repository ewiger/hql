// Presentation formats: text with a known syntax, each a subtype of String.
// A value of a format type is text that parses in that format.
//
// See doc/proposals/HQL-0017/README.md and doc/wiki/hql/presentation.hmd.

type Text     : String
// HQL's own literals: the text a program writes to produce the value.
type Hql      : String
type Markdown : String
// Every HyperMarkDown text is Markdown.
type Hmd      : Markdown
type Json     : String
type Yaml     : String
type Csv      : String
type D2       : String
type Mermaid  : String

// Writing a value in a format, provided by the `format` extension:
// format<T, F : String>(x: T, as: Type<F>, limit: Int = 0, renderer: String = "") : F
// Each shorthand fixes the format and nothing else:
// text     := format(Text)
// hql      := format(Hql)
// markdown := format(Markdown)
// hmd      := format(Hmd)
// json     := format(Json)
// yaml     := format(Yaml)
// csv      := format(Csv)
// d2       := format(D2)
// mermaid  := format(Mermaid)
