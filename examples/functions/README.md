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
| [generic function](queries/generic-function.hql) | `fn first<T>` takes its types from each call; `T : Orderable` is checked there | `[1, 3]` |
| [generic variadic](queries/generic-variadic.hql) | `xs: ...T` binds `T` from the trailing arguments: `pack(1, 2)` is a `List<Int>` | `[[1, 2], [], [5]]` |
| [alias](queries/alias.hql) | `top(n) := sort \| take(n)`: the checker writes the signature | `[1, 2]` |
| [partial application](queries/partial-application.hql) | `add(10, _)` is a function of the argument left open | `[19, 17]` |
| [block body](queries/block-body.hql) | A value block: bindings, then the expression that is the function's value | `[6.0, 12.5]` |
| [block with a Seq result](queries/block-seq-result.hql) | A value block returning `Seq<Int>` without `yield` | `[4, 16]` |
| [block pipeline](queries/block-pipeline.hql) | One pipeline as an `=` body and as a block over several lines | `[12, 12]` |
| [block in a receiver, trait and impl](queries/block-receiver-trait-impl.hql) | A trait default body, an `impl` function and a receiver function, each a block | `["area 6.0", "6"]` |
| [block closure](queries/block-closure.hql) | Block bindings are local; a lambda on the last line captures them and is typed by the result | `[21, 25, 100]` |
| [block query](queries/block-query.hql) | Query steps over a vault as bindings, the last line the result (vault `mapreduce/wiki`) | `["Dune", "Frankenstein", "The Hobbit"]` |
| [invalid parameter type](queries/invalid-parameter-type.hql) | A `String` given to an `Int` parameter is refused at check time | type error |
| [invalid generic variadic mixed](queries/invalid-generic-variadic-mixed.hql) | `pack(1, "owl")`: the trailing arguments fill one `T` | type error |
| [invalid hole in expression](queries/invalid-hole-in-expression.hql) | `_ + 1` is not a whole argument, so it is no hole | syntax error |
| [invalid hole argument](queries/invalid-hole-argument.hql) | A fixed argument beside a hole must fit its parameter, which the error names | type error |
| [invalid block yield and value](queries/invalid-block-yield-and-value.hql) | A block that yields and also ends in an expression | syntax error |
| [invalid block only bindings](queries/invalid-block-only-bindings.hql) | A block with neither a final expression nor a `yield` | syntax error |
| [invalid block for in value](queries/invalid-block-for-in-value.hql) | `for` in a value block | syntax error |
| [invalid block after equals](queries/invalid-block-after-equals.hql) | `= {` begins a record literal, never a block | syntax error |

A function type is written with an arrow, `Int -> Int` or `(Int, Int) -> Int`,
and every diagnostic prints it that way. Parameter positions are
contravariant and the result covariant, so a function that accepts more and
returns less stands where another is expected.
