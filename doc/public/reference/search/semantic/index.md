<!-- hql reference: search/semantic -->
# `search/semantic`

```hql
module semantic
```

Semantic retrieval: a corpus ranked against a query by vectors a
language model made ahead of time, read from the index that
`[semantics] index` in hql.toml names. The query is looked up in the
index too; one the index does not hold is an evaluation failure. Imported where it is
used: `import semantic`.

- [`semantic`](semantic.md) — Semantic retrieval: a corpus ranked against a query by vectors a language model made ahead of time, read from the index that `[semantics] index` in hql.toml names.
<!-- hql reference: end -->
