// status: invalid
// feature: An argument declared positional only is refused by name
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `lib/std/collections/mod.hql` declares `fn count<T>(xs: Collection<T>, value: T, /)`:
// the `/` ends the parameters a call supplies by position. `count(1)` counts
// the ones; naming the value is refused.
[1, 2, 1] | count(value = 1)
