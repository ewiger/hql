<!-- hql reference: std/format.DiagnosticText -->
# `DiagnosticText`

In [`std/format`](../index.md).

```hql
type DiagnosticText : Format
```

The text form of a `Diagnostic` (HQL-0045), which writes a diagnostic,
or a sequence of them, and nothing else: each one's heading,
`phase severity[kind]: message`, its origin and the fields that apply,
then a count per severity when there is more than one. It is written
only.
<!-- hql reference: end -->
