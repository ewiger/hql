# Stats: means, spread, quantiles, the mode and correlation

The cases follow [HQL-0041 part 2](../../doc/proposals/HQL-0041/parts/02-stats.md).
The `stats` module is imported with `import stats`. A statistic of `Int` or
`Float` values is a `Float`, and a statistic of `Decimal` values is a
`Decimal`, exact except where a quotient does not terminate. A statistic
undefined over its input is `none`. Every expected value is the one Python's
`statistics` gives for the same input, `quantiles` with `method="inclusive"`.

The prelude's `variance` is the population variance of `Int` and `Float`
values; over those, a program importing `stats` writes `stats.variance` for
the sample variance.

| Case | What it shows | Result |
| --- | --- | --- |
| [mean](queries/mean.hql) | the mean of `Int` values | `5.0` |
| [spread](queries/spread.hql) | `stats.variance`, `stdev`, `pvariance` and `pstdev` | `[4.571428571428571, 2.138089935299395, 4.0, 2.0]` |
| [exact](queries/decimal-exact.hql) | statistics of `Decimal` prices stay `Decimal` | `[3.22, 2.0806, 2.60075]` |
| [percentile](queries/quantile.hql) | the 95th percentile, exact | `4.9300` |
| [quartiles](queries/quartiles.hql) | `quantiles(4)` | `[4.0, 4.5, 5.5]` |
| [median](queries/median.hql) | the median of an even count | `2.5` |
| [mode](queries/mode-tie.hql) | a tie resolves to the least value | `1` |
| [correlation](queries/correlation.hql) | `covariance` and `correlation` of pairs | `[3.2, 0.7917946548886297]` |
| [undefined](queries/empty.hql) | the mean of nothing, the `stdev` of one value, a constant side | `[none, none, none]` |
| [quantile out of range](queries/invalid-quantile.hql) | `quantile(1.5)` | an evaluation error |
| [one group](queries/invalid-quantiles.hql) | `quantiles(1)` | an evaluation error |
| [ambiguous variance](queries/ambiguous-variance.hql) | a bare `variance` over `Int`s with `stats` imported | a check error |
| [not imported](queries/not-imported.hql) | `stdev` without `import stats` | a check error |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/stats/queries/spread.hql
```
