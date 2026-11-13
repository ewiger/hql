# Decimal, an exact decimal number

The cases follow [HQL-0031](../../doc/proposals/HQL-0031/README.md): a number
written with a decimal point is a `Decimal`, an arbitrary-precision decimal
whose value is exactly its digits, and a `Float` is written with the suffix
`f`. A `Decimal` keeps its scale for display and compares by value; an `Int`
beside one widens to it, and a `Float` beside one is refused.

| Case | What it shows | Result |
| --- | --- | --- |
| [exact sum](queries/exact-sum.hql) | `0.1 + 0.2 == 0.3` | `true` |
| [scale kept](queries/scale-kept.hql) | a sum has the larger scale, a product the sum of the scales | `[1.50, 2.75, 2.250, 0.02]` |
| [long literal](queries/long-literal.hql) | thirty-nine significant digits, each kept through a sum | `123456789012345678901234567890.123456790` |
| [Int widening](queries/int-widening.hql) | `12.50 * 3` and `10 / 4.0`: an Int is a Decimal of scale 0 | `[37.50, 2.5, 6.75]` |
| [widening in a comparison](queries/widening-compare.hql) | `12.5 < 13` and `1.0 == 1.00` compare by value | `[true, true, true, true]` |
| [round half to even](queries/round-half-even.hql) | `round(2.345, 2)` is `2.34` and `round(2.355, 2)` is `2.36` | `[2.34, 2.36, 2, 4, 1.200]` |
| [conversions](queries/conversions.hql) | `decimal`, `float`, `int` and `to_decimal` | five texts |
| [JSON decode](queries/json-decode.hql) | a JSON number read as a `Decimal` field keeps every digit | `[1234567890.123456789012345, 0.10]` |
| [JSON and YAML encode](queries/json-encode.hql) | a `Decimal` is written as a number with its scale | `12.50` in both |
| [CSV decode](queries/csv-decode.hql) | a CSV cell read as a `Decimal` field, summed exactly | `0.60` |
| [Float mixing](queries/invalid-float-mix.hql) | `1.5 + 2.0f` is refused, naming `decimal(x)` and `float(d)` | a type error |
| [division by zero](queries/invalid-division-by-zero.hql) | `1.50 / 0` | an evaluation error |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/decimal/queries/exact-sum.hql
```
