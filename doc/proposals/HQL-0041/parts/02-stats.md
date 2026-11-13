# Part 2: `stats`

- **Depends on**: [the proposal](../README.md) for ownership;
  [HQL-0014](../../HQL-0014/README.md) for reductions and combiners;
  [HQL-0036](../../HQL-0036/README.md) for partitioning
- **Needed by**: none

## Scope

The descriptive statistics over a collection of numbers, the ground Python's
`statistics` covers. `collections` already holds `sum`, `min`, `max` and
`count`; `stats` holds the rest.

## Declarations

`N` stands for each of `Int`, `Float` and `Decimal`: every operation with an
`N` is declared three times, once per type, as `sum` is.

```hql
// lib/std/stats/mod.hql
module stats
import core

reduction fn mean(self: Collection<Int>) : Option<Float>
reduction fn mean(self: Collection<Float>) : Option<Float>
reduction fn mean(self: Collection<Decimal>) : Option<Decimal>

reduction fn variance(self: Collection<N>) : Option<R>    // sample, n - 1
reduction fn stdev(self: Collection<N>) : Option<R>       // sample, n - 1
reduction fn pvariance(self: Collection<N>) : Option<R>   // population, n
reduction fn pstdev(self: Collection<N>) : Option<R>      // population, n

fn median(self: Collection<N>) : Option<R>
fn quantile(self: Collection<N>, q: Float) : Option<R>
fn quantiles(self: Collection<N>, n: Int) : List<R>
fn mode<T : Orderable>(self: Collection<T>) : Option<T>

reduction fn covariance(self: Collection<Pair<N, N>>) : Option<R>
reduction fn correlation(self: Collection<Pair<N, N>>) : Option<Float>
```

- `R` MUST be `Float` where `N` is `Int` or `Float`, and `Decimal` where `N`
  is `Decimal`. A statistic of exact numbers stays exact up to `Decimal`'s
  division, which rounds half-even to 100 significant digits
  ([HQL-0031](../../HQL-0031/README.md)).
- `stdev` and `pstdev` over `Decimal` MUST be the square root of the
  variance rounded the same way.

## Empty and short inputs

Absence is the answer a statistic gives where it is undefined.

- `mean`, `pvariance`, `pstdev`, `median`, `quantile` and `mode` MUST give
  `none` over an empty collection.
- `variance`, `stdev` and `covariance` MUST give `none` over fewer than two
  values.
- `correlation` MUST give `none` over fewer than two pairs, or when either
  side has zero variance.
- `quantiles` MUST give the empty list over an empty collection.
- A `Float` input holding NaN MUST make every statistic over it NaN; an
  infinite input follows IEEE 754 arithmetic.

## Definitions

Each statistic is pinned to one formula.

```text
mean(x)       = sum(x) / n
pvariance(x)  = sum((x_i - mean)^2) / n
variance(x)   = sum((x_i - mean)^2) / (n - 1)
covariance(p) = sum((x_i - mean_x) * (y_i - mean_y)) / (n - 1)
correlation   = covariance / (stdev_x * stdev_y)            Pearson's r

quantile(x, q), x sorted ascending as s_0 .. s_(n-1), 0 <= q <= 1:
  h = (n - 1) * q
  quantile = s_floor(h) + (h - floor(h)) * (s_(floor(h)+1) - s_floor(h))
median(x)       = quantile(x, 0.5)
quantiles(x, k) = [quantile(x, i / k) for i in 1 .. k - 1]
```

- `quantile` MUST be the linear interpolation above, Hyndman and Fan's type
  7. It is the default of R, NumPy and spreadsheets, and Python's `quantiles`
  with `method="inclusive"`.
- `quantile` with `q` outside `0..1`, and `quantiles` with `k < 2`, MUST be
  an evaluation error.
- `mode` MUST give the most frequent value, and among equally frequent
  values the least by the type's order. Python returns the first one met;
  a collection has no first, so the order decides.

## Execution

What an executor may split follows from whether a statistic merges.

- `mean`, `variance`, `stdev`, `pvariance`, `pstdev`, `covariance` and
  `correlation` MUST be reductions: each partition computes `(n, mean, M2)`,
  and partitions combine by Chan's pairwise update.

```text
combine((n_a, mean_a, M2_a), (n_b, mean_b, M2_b)):
  n     = n_a + n_b
  delta = mean_b - mean_a
  mean  = mean_a + delta * n_b / n
  M2    = M2_a + M2_b + delta^2 * n_a * n_b / n
```

- Over `Float`, a reduction's result MAY differ in its last bits between two
  partitionings of one input; over `Decimal` it MUST NOT.
- `median`, `quantile`, `quantiles` and `mode` MUST be plain `fn`s, which are
  barriers: each needs the whole collection, and no combiner of partial
  results gives the exact answer.

## Examples

```hql
import stats

// Mean and spread of the counts, per species.
source(file("counts.csv"), Csv)
| group(by = r => r.species)
| map(g => {
    species: g.key,
    mean:    g.items | map(r => r.count) | mean,
    stdev:   g.items | map(r => r.count) | stdev,
  })

// The 95th percentile of prices, exact.
source(file("prices.csv"), Csv) | map(r => r.price) | quantile(0.95)

// How strongly two columns move together.
rows | map(r => Pair(first = r.length, second = r.weight)) | correlation
```

## Test plan

- Each statistic against Python's `statistics` over a fixed list of `Int`,
  `Float` and `Decimal`, `quantile` against `quantiles(method="inclusive")`.
- Empty and single-value inputs give `none` as listed; zero variance gives
  `none` for `correlation`; NaN propagates.
- `mode` ties resolve to the least value.
- A reduction over one input split into 1, 2 and 7 partitions gives the same
  `Decimal` result each time, and `Float` results within 4 ulps.
- `quantile(1.5)` and `quantiles(1)` are refused.
