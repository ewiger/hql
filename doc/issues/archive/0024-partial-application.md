# 0024 — Partial application

Status: done
Source: [0010](0010-yield-and-bind.md), which no longer carries `bind`

Fixing some arguments of a function and getting back a function of the rest
is written today as a lambda that captures a binding:

```hql
add = (a: Int, b: Int) => a + b
k = 10
[1, 2] | map(n => add(k, n))      // [11, 12]
```

That works and stays. This issue decides whether HQL gets a shorter form, and
if so, which one. The [queries model](../../models/behavior/queries.md) places the
question in the function design, not in query sources: section "Partial
application is not part of the source model".

## Resolution

Partial application is written with a hole: `_` as a whole argument of a call
makes the call a function of its holes, `add(10, _) : Int -> Int`. The parser
writes the call as a lambda over its holes, and each hole takes the type of
the parameter it fills, so it needs neither generic functions nor an overload
per arity. The [type system](../../wiki/hql/type-system.hmd) card states it,
section "Function types"; `examples/functions/` covers it, with `invalid`
cases for a hole inside an expression and an argument that does not fit.
