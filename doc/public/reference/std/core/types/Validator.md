<!-- hql reference: std/core.Validator -->
# `Validator`

In [`std/core`](../index.md).

```hql
trait Validator
```

A check a value passes: a trait extending `Validator` is a validator
type. `validate f with V` holds where `V`'s `validate` of `f` is true,
and a field declared `f: V` reads as the type implementing `V`, held to
it.

## Members

```hql
fn validate(self) : Bool
```
<!-- hql reference: end -->
