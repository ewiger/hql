# Functions: declared, bound, passed and returned

Pure programs, no vault. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/functions/queries/named-function.hql
```

| Query | What it shows | Result |
| --- | --- | --- |
| [named function](queries/named-function.hql) | `fn` with typed parameters, called recursively and passed to `map` | `[1, 2, 6, 24, 120]` |
| [bound lambda](queries/bound-lambda.hql) | A lambda bound to a name, typed by annotation or by the binding | `[5, 7, 9]` |
| [function parameter](queries/function-parameter.hql) | A function as a parameter, and one returned with its captured value | `36` |
| [variadic](queries/variadic.hql) | `xs: ...Int` collects zero or more trailing arguments | `[0, 3, 10]` |
| [wider parameter](queries/wider-parameter.hql) | A `Data -> Int` function where an `Int -> …` one is expected | `[1, 1]` |
| [invalid parameter type](queries/invalid-parameter-type.hql) | A `String` given to an `Int` parameter is refused at check time | type error |

A function type is written with an arrow, `Int -> Int` or `(Int, Int) -> Int`,
and every diagnostic prints it that way. Parameter positions are
contravariant and the result covariant, so a function that accepts more and
returns less stands where another is expected.
