# Aliases and holes

## Aliases

Most small functions are a pipeline fragment with a name. `:=` defines an
**alias**, and the checker writes its type: the input is the most general one
the fragment accepts, and each parameter takes the type of the step argument
it fills.

```hql
top(n) := sort | take(n)
[412, 280, 310, 145] | top(2)
```

```text
[145, 280]
```

`hql check` prints the signature it wrote:

<!-- run: check -->
```hql
top(n) := sort | take(n)
1
```

```text
top : (Collection<T>, Int) -> List<T> where T : Orderable
Int
```

## Holes

A call with `_` for an argument is a function of the arguments left open — a
**partial application**:

```hql
fn add(a: Int, b: Int) : Int = a + b
[1, 2, 3] | map(add(10, _))
```

```text
[11, 12, 13]
```

A hole is a whole argument. `_ + 1` is not a hole, and a lambda, `n => n + 1`,
writes that function.
