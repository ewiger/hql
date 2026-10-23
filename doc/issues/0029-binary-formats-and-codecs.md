# 0029 — Binary formats, and codecs an extension or a package provides

Status: todo
Related: [HQL-0024 part 7](../proposals/HQL-0024/parts/07-formats-and-codecs.md),
[encode, decode and format](../proposals/HQL-0024/encode-decode-format.md),
[HQL-0024 part 9](../proposals/HQL-0024/parts/09-packages-and-extensions.md),
[presentation](../wiki/hql/presentation.hmd)

`Format` is a category of its own and every format today is textual: `Text`,
`Lines`, `Json`, `JsonLines`, `Yaml`, `Csv`, `Markdown`, `Hmd`, `Hql`, `D2`,
`Mermaid`. The codec contracts, `Decoder<F, T>` and `Encoder<T, F>`, are
declared in `std/format/mod.hql` and implemented by the binary alone. Two things
follow from the format model and are not built:

- **Binary formats** as a format kind: `Avro`, `Parquet`, `Protobuf`. A format
  need not be text; a binary format decodes bytes to values and encodes values
  to bytes, record-oriented or document-oriented, with a schema where the
  format has one.
- **Codecs from outside the binary**: an extension or a package declares that
  it implements `Decoder<F, T>` or `Encoder<T, F>` for a format it brings, and
  the registry binds the contract to that implementation, as part 9 has it for
  modules.

## Scope

- The format kind: what a binary format is to `source`, `tap`, `target` and
  `sink`, how a schema is named, and what `format(x, Parquet)` means in memory,
  where the encoded representation is bytes and not a `String`.
- The contract side: an `impl Decoder<Avro, Data> for …` written as an
  extension descriptor, or in HQL once generic traits admit an `impl`.
- The implementation: one binary codec, end to end, with a test over a file
  written by another tool.

## Done when

- One binary format reads and writes through the existing I/O vocabulary.
- A codec provided outside the core is bound through the registry, not by
  editing the renderer table.
- Part 7's work unit 4 is marked done and the presentation card lists the
  format.
