# Generic traits: type parameters on `trait` and `impl`

Pure programs, no vault. Each case pins the type and result
[HQL-0027](../../doc/proposals/HQL-0027/README.md) specifies, and checks
today. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/generic-traits/queries/declared-index.hql
```

| Query | What it shows | Result |
| --- | --- | --- |
| [declared index](queries/declared-index.hql) | `impl<T> Index<T> for Ring<T>`: a generic declared type takes `xs[i]` | `["raven", "robin", "wren"]` |
| [bound inference](queries/bound-inference.hql) | `E : Index<T>` leaves `T` to the implementation each call reaches | `[10, 3, null]` |
| [built-in index](queries/builtin-index.hql) | A List satisfies `Index<T>` through the native `impl<T> Index<T> for Seq<T>` | `[20, 7, null]` |
| [trait arguments](queries/trait-arguments.hql) | `Scale<Int>` and `Scale<Float>` for one type, chosen by the argument | `[3.0, 0.75]` |
| [codec shape](queries/codec-shape.hql) | The representation is `Self`, the value read is the parameter | `[["raven", "robin"], ["wren"]]` |

A trait's parameter is the only way it names another type. Where the
implementing type decides it, as the element type of `Index<T>`, a call
leaves it open and inference fills it from the one implementation that
applies; where a type has several, as `Scale<F>`, the call's arguments
choose.

The codec contracts themselves, `Decoder<T>` and `Encoder<T>` over a
format, are pinned against the runtime's decoder and encoder tables by
`tests/extension_contracts.rs`: a program cannot yet hold a format as a
value, so no program in this corpus calls them.
