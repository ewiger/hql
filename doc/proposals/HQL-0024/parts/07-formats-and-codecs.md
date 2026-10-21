# Part 7: Formats and codecs

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 60 to
  65 and 49 to 52 of the Decision section
- **Depends on**: [the standard library](03-standard-library.md) for the
  `format.*` modules
- **Needed by**: [packages](09-packages-and-extensions.md) (a package may
  provide codecs)

## Scope

What a **format** is, what it is not, and how values cross between HQL and an
external representation.

## Rules

### A format is a representation descriptor

`Text`, `Lines`, `Json`, `Yaml`, `Csv`, `Markdown`, `Avro`, `Parquet` describe
representation and framing. A format is not a string value, and it does not say
whether what is read is finite or streaming: `source(file("x.txt"), Lines)`
yields finite lines and `source(stdin, Lines)` yields a stream, because the
endpoint decides boundedness and the format decides interpretation
([HQL-0021](../../HQL-0021/README.md), which this agrees with).

### What is refused

- `Json : String` and `Lines : String`: a descriptor is not text.
- `Lines : Seq<String>`: framing is not the shape of the result.
- `Format<T>` as the model: one format encodes many value structures, so type
  compatibility is a codec's business, not a type parameter's.

### Codecs

A **codec contract** connects HQL values, a format and the external bytes. It
is declared in HQL and may be implemented natively, following the rule that
the HQL declaration is authoritative. Extensions may contribute codecs.

## Current state

- [HQL-0017](../../HQL-0017/README.md), accepted and implemented: a
  presentation format is a **type of text with a known syntax, a subtype of
  `String`**. [`std/format.hql`](../../../../std/format.hql) declares
  `type Json : String`, `type Hmd : Markdown`, and
  `format<T, F : String>(x: T, as: Type<F>, ...) : F` returns text of that
  type, so a step after `format(Json)` reads a `Json` value that is a `String`.
  The shorthands `json`, `yaml`, `csv`, `d2`, `mermaid` are `format(F)`.
- [HQL-0021](../../HQL-0021/README.md), drafted: the same format types name
  how an endpoint's bytes are documents; `source(e, Json)` decodes and
  `sink(xs, e, Json)` encodes.
- `Lines`, `Avro` and `Parquet` do not exist. `Hql`, `D2` and `Mermaid` exist
  and the draft does not list them.

## What changes

This is the one part where the draft **contradicts an accepted and implemented
record**. Adopting it means:

- Format types leave the `String` lattice. `format(x, Json)` can no longer
  return a `Json` that is also a `String`; either `format` returns `String`
  and the format is an argument only, or a formatted value is a distinct
  `Encoded<F>`-like value with its own path back to text. HQL-0017's rule that
  "a value of a format type is text that parses in that format" is withdrawn.
- The renderer table of the [presentation card](../../../wiki/hql/presentation.hmd)
  becomes a set of codec contracts: a renderer is the encode half of a codec
  for one value type.
- `Lines` is added as a framing format, and `format.lines` as a module.
- `Avro` and `Parquet` are binary formats; the first binary formats in the
  language, which fixes that a format need not be text.

The alternative is to keep HQL-0017 and reject this part: formats stay text
types, `Lines` is added as `type Lines : Text` with a line-per-document
framing rule in HQL-0021, and codecs are what HQL-0021 already calls decoding
and encoding. The README records which way the proposal goes; until then the
accepted record stands and this part is the draft's position.

## Work units

1. The decision between the two models above, recorded in the README and in
   HQL-0017's status if it is superseded.
2. If adopted: the codec contract declaration in `std/format/`, the renderer
   migration, and the `Lines` format in HQL-0021.
3. Binary formats as a format kind.
