# Structural types, open views and contracts

The cases follow [HQL-0032](../../doc/proposals/HQL-0032/README.md), which
supersedes HQL-0030: a plain
structural type is closed, one narrowing `Data` (or `DataContract`) is an open
view, and `data | T` reads a `Data` as `T` without changing it. A
`TypeContract` or `DataContract` adds rules, `require P` and
`validate f with V`, and field defaults, `age: Int = 18`: every value it
reads holds every rule.

| Case | What it shows | Result |
| --- | --- | --- |
| [open view](queries/open-view.hql) | `User : Data` reads `id` and `name`, keeps `phone`, and `\| Contact` reads it | `["Alice", "+41"]` |
| [closed point](queries/closed-point.hql) | `type Point = {…}` reads a tree with exactly its fields | a `Point` |
| [closed, extra field](queries/invalid-closed-extra-field.hql) | a closed type refuses an undeclared `z` when the `Data` is read | an evaluation error |
| [decoded sightings](queries/decoded-sightings.hql) | `source(file(…), Json) \| Sighting`: decoding at the source, interpretation in the pipeline | the first two species |
| [empty map](queries/empty-map.hql) | `{:}` takes a declared `Map<String, Int>` type | `true` |
| [contract rules](queries/contract-rules.hql) | `require` and `validate … with` hold for each value `\| UserData` reads; `age?` reads an optional field | `["Alice", "CH"]` |
| [presence and implication](queries/contract-presence.hql) | `email? or phone?` tests presence, and `=>` is implication | `["open", "closed"]` |
| [contract defaults](queries/contract-defaults.hql) | `age: Int = 18` stands in where the data has no `age` | `[18, 34]` |
| [broken rules](queries/invalid-contract-rule.hql) | a value that breaks rules does not read, and each broken rule is named as written | an evaluation error |
| [rule on a plain type](queries/invalid-rule-on-plain-type.hql) | only a `TypeContract` or `DataContract` has rules | a type error |
| [constructor](queries/contract-constructor.hql) | `UserData(id = "u-1")` fills `age` with its default and holds the rules | `[18, 34]` |
| [constructor, broken rule](queries/invalid-contract-constructor.hql) | a constructor is held to the rules, as `\| T` is | an evaluation error |
| [inherited fields and rules](queries/contract-inheritance.hql) | `Window : Span` has `low` and `high`, and its rule reads them | `[5, 9]` |
| [inherited rule broken](queries/invalid-inherited-rule.hql) | a value read as `Window` holds `Span`'s rule | an evaluation error |
| [collection defaults](queries/contract-collection-defaults.hql) | `tags: List<String> = []` | `[0, 2]` |
| [default with a call](queries/invalid-default-call.hql) | a default is a literal, never a call | a type error |
| [warnings and names](queries/contract-warnings.hql) | `warn age? < 120 as plausible_age` leaves a value of 130 valid | `[34, 130]` |
| [validation results](queries/validate-results.hql) | `xs \| validate(UserData)` gives a `Result` per value; `oks()` and `errs()` split them, and each `Violation` names its rule | the valid ids, the broken rules |
| [validation in a match](queries/validate-match.hql) | `match d \| validate(UserData) { ok(u) => …, err(e) => … }` | `"1 violation of UserData"` |
| [violation fields](queries/validate-violation-fields.hql) | one `Invalid` holds every violation; a `missing` or `type` violation fills `expected` and `actual` | the rules, types and trees |
| [original value](queries/validate-original-value.hql) | `e.value` is the `Data` validated, before defaults | the two rejected trees |
| [one element per violation](queries/validate-flat-violations.hql) | `errs() \| flat_map(e => e.violations)` | three violations |
| [stored `Invalid`](queries/stored-invalid.hql) | `stored \| Invalid` reads a written failure back, `severity` defaulting to `"error"` | `["UserData UserData.id_present error"]` |
| [results in collections](queries/results-in-collections.hql) | `map`, `map_err` and `match` over each `Result` of a `List`, a `Seq` and a `Stream` | the ids, `-` or `not UserData` per value |
| [`oks` and `errs` over a stream](queries/stream-oks-errs.hql) | over a `Stream` of results each keeps a stream, bounded afterwards by `take(n)` | `[["u-1", "u-3", "u-1"], ["UserData.id_present", "UserData.id_present"]]` |
| [validating a closed value](queries/invalid-validate-closed-value.hql) | `validate(T)` reads `Data` or an open record | a type error |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/structural/queries/open-view.hql
```
