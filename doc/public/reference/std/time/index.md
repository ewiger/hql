<!-- hql reference: std/time -->
# `std/time`

```hql
module time
```

Time: zones, the units a value truncates to, and the operations over the
time scalars of `core`: conversion from text, parts, arithmetic,
truncation, rendering, and the evaluation clock. Not in the prelude: a
program writes `import time`.

- [`Zone`](types/Zone.md) — A time zone: an IANA name the time-zone database holds, `"Europe/Berlin"`, `"UTC"`, or a fixed offset `"+02:00"`.
- [`TimeUnit`](types/TimeUnit.md) — The units a time value truncates to.
- [`Nanosecond`](types/Nanosecond.md) — The units a time value truncates to.
- [`Microsecond`](types/Microsecond.md) — The units a time value truncates to.
- [`Millisecond`](types/Millisecond.md) — The units a time value truncates to.
- [`Second`](types/Second.md) — The units a time value truncates to.
- [`Minute`](types/Minute.md) — The units a time value truncates to.
- [`Hour`](types/Hour.md) — The units a time value truncates to.
- [`Day`](types/Day.md) — The units a time value truncates to.
- [`Week`](types/Week.md) — The units a time value truncates to.
- [`Month`](types/Month.md) — The units a time value truncates to.
- [`Quarter`](types/Quarter.md) — The units a time value truncates to.
- [`Year`](types/Year.md) — The units a time value truncates to.
- [`to_date`](to_date.md) — `self` read as an ISO 8601 date, `some(Date)`, or `none` where it is not one: `"2026-10-24".to_date()` is `some(2026-10-24)`.
- [`to_datetime`](to_datetime.md) — `self` read as an ISO 8601 date-time with no offset, or `none`.
- [`to_instant`](to_instant.md) — `self` read as an RFC 3339 instant, with `Z` or an offset, or `none`.
- [`to_duration`](to_duration.md) — `self` read as an ISO 8601 duration without years or months, or `none`.
- [`parse_date`](parse_date.md) — `self` read through a strftime `pattern` as a `Date`, or `none` where it does not match: `"24.10.2026".parse_date("%d.%m.%Y")`.
- [`parse_datetime`](parse_datetime.md) — `self` read through a strftime `pattern` as a `DateTime`, or `none`.
- [`parse_instant`](parse_instant.md) — `self` read through a strftime `pattern` as an `Instant`, or `none`; the pattern writes an offset with `%z` or a zone with `%Z`.
- [`at`](at.md) — The wall-clock reading of `self` in `zone`.
- [`in_zone`](in_zone.md) — The instant the wall-clock reading `self` names in `zone`.
- [`offset`](offset.md) — The offset from UTC `self` has at the instant `at`: `PT2H` in Berlin in summer.
- [`year`](year.md) — The year of `self`.
- [`month`](month.md) — The month of `self`, `1..12`.
- [`day`](day.md) — The day of the month of `self`, `1..31`.
- [`weekday`](weekday.md) — The ISO day of the week of `self`: Monday is 1, Sunday is 7.
- [`day_of_year`](day_of_year.md) — The day of the year of `self`, `1..366`.
- [`iso_week`](iso_week.md) — The ISO week of `self`, `1..53`, in the year `iso_year` gives.
- [`iso_year`](iso_year.md) — The year the ISO week of `self` belongs to: `2025` for `2024-12-30`, the Monday of the first ISO week of 2025.
- [`date`](date.md) — The date of `self`.
- [`hour`](hour.md) — The hour of `self`, `0..23`.
- [`minute`](minute.md) — The minute of `self`, `0..59`.
- [`second`](second.md) — The second of `self`, `0..59`.
- [`nanosecond`](nanosecond.md) — The nanoseconds of `self` past its second.
- [`unix_seconds`](unix_seconds.md) — The whole seconds from `1970-01-01T00:00:00Z` to `self`, rounded toward negative infinity.
- [`unix_millis`](unix_millis.md) — The whole milliseconds from `1970-01-01T00:00:00Z` to `self`, rounded toward negative infinity.
- [`total_seconds`](total_seconds.md) — The length of `self` in seconds, exactly: `PT1.5S` is `1.5`.
- [`total_millis`](total_millis.md) — The length of `self` in whole milliseconds, rounded toward zero.
- [`plus`](plus.md) — `self` moved later by `by`, earlier where `by` is negative.
- [`minus`](minus.md) — `self` moved earlier by `by`.
- [`since`](since.md) — The duration from `earlier` to `self`, negative where `earlier` is after `self`.
- [`add_days`](add_days.md) — `self` moved by `n` days.
- [`add_months`](add_months.md) — `self` moved by `n` months, clamped to the last day of the month it lands in: `Date("2026-01-31").add_months(1)` is `2026-02-28`.
- [`add_years`](add_years.md) — `self` moved by `n` years, clamped as `add_months` is: February 29 moved by a year is February 28.
- [`days_since`](days_since.md) — The days from `earlier` to `self`, negative where `earlier` is after it.
- [`negate`](negate.md) — `self` with its sign reversed.
- [`times`](times.md) — `self` multiplied by `n`.
- [`truncate`](truncate.md) — The start of the `unit` containing `self`: the first day of the month for `Month`, the Monday of its ISO week for `Week`.
- [`dates`](dates.md) — Every date from `self` up to and excluding `end`, as `range` gives the `Int`s; empty where `end` is not after `self`.
- [`fmt`](fmt.md) — `self` written through a strftime `pattern`, the directives `parse_date` reads: `Date("2026-10-24").fmt("%d %B %Y")` is `"24 October 2026"`.
- [`now`](now.md) — The evaluation instant: the host's clock, or `--now`.
- [`today`](today.md) — The date `now()` falls on in `zone`, or in `local_zone()`.
- [`local_zone`](local_zone.md) — The evaluation zone: `--zone`, else the `TZ` environment variable, else the host's, else `UTC`.
<!-- hql reference: end -->
