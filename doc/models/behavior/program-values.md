# Program values and execution contexts

Status: accepted design direction; **not implemented beyond the single-expression
bootstrap**. This decision does not expand the grammar in expressions.md.

An expression evaluates to a typed value. A program returns the value of its
last expression. Earlier unbound expression values are evaluated and discarded;
bindings retain values. A declaration yields a no-visible-result value, called
Unit in the corpus; Unit versus Void spelling remains unresolved.

```hql
alice : Doc = [[alice]]

title : String = alice.title
header = alice.header

alice.body
```

The program result is the body value. It does not implicitly print title,
header and body. Preferred typed and inferred bindings do not require `let`.
The doc reference `[[alice]]`, expression ascription `[[alice]] : Doc` and typed
binding `alice : Doc = [[alice]]` are distinct constructs.

The execution context consumes the returned value. The standalone CLI currently
chooses to print its one expression result. Future hosts can instead retain it,
render it, validate it or pass it to another computation. Explicit `print` is a
proposed effect; pipeline application does not make effects pure. Pure updates
construct a modified Card; persistence needs a separate explicit effect.

## HMD cell contexts

Cells are closures over the defining card environment: this:Card, explicit
header imports, and explicit card-level bindings. Cards never implicitly
export variables named after themselves. Cell-local bindings/imports may shadow
those names without altering another cell's closure. Transclusion retains the
defining closure environment. Order, dependency cycles and cache invalidation
remain open; these choices must not silently change lexical scope.

- `hql#eval` evaluates arbitrary HQL code; the host may insert the returned
  value using its type, including String or Hmd.
- `hql#query` returns a value the host presents, such as Seq<Card> or Graph.
- `hql#declare` contributes bindings/knowledge under an explicit host contract;
  its Unit-like result normally renders nothing. Contribution transaction and
  replay rules are unresolved.

Typed presentation values such as HmdInline, HmdTable, HmdList, HmdTree and
HmdGraph are candidates. Graph is a data projection; HmdGraph is a presentation
value; neither implicitly renders itself. Embedded evaluation should initially
be pure; any effects need an explicit execution policy.

See [corpus](../../../examples/README.md), [current grammar](expressions.md),
and [knowledge model](../domain/knowledge.md).
