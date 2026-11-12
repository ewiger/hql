# Time: Date, DateTime, Instant and Duration

The cases follow [HQL-0041 part 1](../../doc/proposals/HQL-0041/parts/01-time.md).
`core` declares four time scalars with no literal: a `Date`, a `DateTime`
with no zone, an `Instant` on the UTC timeline, and a `Duration`. Each is
built by its constructor from ISO 8601 text or from named parts, compares and
sorts by time, and is written by every format as its ISO 8601 text. The
operations over them are the `time` module's, imported with `import time`.

| Case | What it shows | Result |
| --- | --- | --- |
| [construction](queries/construction.hql) | `Date(text)`, `Date(year =, month =, day =)` and `to_date` | three dates |
| [sorting](queries/sort-dates.hql) | dates sort by time | `[2025-12-31, 2026-01-01, 2026-10-24]` |
| [instant offsets](queries/instant-offset.hql) | `09:30+02:00` and `07:30Z` are one instant | `true` |
| [duration sum](queries/duration-sum.hql) | `Duration` takes `+` | `PT1H45M` |
| [month ends](queries/month-end.hql) | `add_months` and `add_years` clamp to the month's last day | `[2026-02-28, 2024-02-29, 2025-02-28]` |
| [DST gap](queries/dst-gap.hql) | a Berlin reading that does not happen moves forward by the gap | `2026-03-29T01:30:00Z` |
| [truncation](queries/truncate.hql) | `truncate` to `Week`, `Month`, `Quarter` and `Year` | four dates |
| [patterns](queries/pattern.hql) | `fmt` and `parse_date` with a strftime pattern | a text and `"true"` |
| [conversion](queries/conversion-none.hql) | `to_date` gives `none` for February 30 | `[none, 2026-02-28]` |
| [JSON decode](queries/json-decode.hql) | declared `Date`, `Instant` and `Duration` fields read their text | two events of October |
| [invalid text](queries/invalid-date-text.hql) | `Date("2026-02-30")` | an evaluation error |
| [invalid sum](queries/invalid-add-dates.hql) | `+` over two dates | a type error |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/time/queries/dst-gap.hql
```
