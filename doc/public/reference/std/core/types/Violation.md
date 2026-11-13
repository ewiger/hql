<!-- hql reference: std/core.Violation -->
# `Violation`

In [`std/core`](../index.md).

```hql
type Violation : DataContract { contract : String path : List<String> kind : String rule : String text : String expected? : String actual? : Data severity : String = "error" message : String }
```

One reason a value does not read as a contract, as `validate(T)`
reports it. `kind` is `"missing"`, `"undeclared"`, `"type"` or
`"rule"`; `rule` is the rule's identity, `Contract.name`, or
`Contract.n` for the n-th unnamed rule; `path` leads from the value to
the field, `[]` for the value itself.
<!-- hql reference: end -->
