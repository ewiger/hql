# 0076 — `parse_date` and its peers read ISO text without a pattern

Status: done
Related: [HQL-0041 part 1](../../proposals/HQL-0041/parts/01-time.hmd),
[`lib/std/time`](../../../lib/std/time/mod.hql),
[`src/extensions/time.rs`](../../../src/extensions/time.rs),
[0064](0064-date-and-the-time-module.md)

`parse_date`, `parse_datetime` and `parse_instant` each require a strftime
`pattern`, so `"1982-05-09".parse_date()` is a type error,
`` `parse_date` needs `pattern` ``. The reading with no pattern is the ISO
one, which `to_date`, `to_datetime` and `to_instant` already give.

A default pattern cannot state that reading: the strftime directives have
no optional fraction of a second, and `%z` reads `+0200` but not `Z` or
`+02:00`, so no pattern reads every text `str` of an `Instant` writes.
Each of the three gains a form of one argument instead, overloaded by
arity, that reads exactly what the matching `to_*` reads.

## Work

- `lib/std/time/mod.hql`: `fn parse_date(self: String) : Option<Date>`,
  and the same for `parse_datetime` and `parse_instant`, each documented
  as the reading of its `to_*`.
- `src/extensions/time.rs`: a call without `pattern` reads through the
  ISO conversion.
- HQL-0041 part 1: the three signatures and the rule that the form without
  a pattern equals the `to_*` conversion.
- An example under `examples/time/`, a case in `tests/time.rs`, the
  reference pages regenerated, the learn page on time, the changelog.

## Done when

- `"1982-05-09".parse_date() == "1982-05-09".to_date()`, and the same for
  `parse_datetime` and `parse_instant`, including text that is not ISO,
  which gives `none`.

## Outcome

Done. Each of the three is overloaded by arity in `lib/std/time`; the
native reads through the ISO conversion where no `pattern` is passed.
HQL-0041 part 1 states the rule, `tests/time.rs` and
[the example](../../../examples/time/queries/parse-without-pattern.hql) pin
it, and the reference pages and the learn page on time show the form.
