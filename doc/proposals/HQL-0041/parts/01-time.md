# Part 1: `time`

- **Depends on**: [the proposal](../README.md) for ownership, the values fixed
  per evaluation, and the rule against silent coercion
- **Needed by**: [HQL-0040](../../HQL-0040/README.md), for file times

## Scope

Five time types and the operations over them: an absolute point on the
timeline, a calendar date and a wall-clock date-time with no zone, a zone, and
a length of time. Python's `datetime`, `time` and `zoneinfo` cover the same
ground.

## Types

Three scalars sit in `core`, beside `Date`; `Zone` and the units are `time`'s.

```hql
// lib/std/core/mod.hql
type Date     : {Scalar, Orderable}  // a proleptic Gregorian calendar date
type DateTime : {Scalar, Orderable}  // a date and a time of day, no zone
type Instant  : {Scalar, Orderable}  // a point on the UTC timeline
type Duration : {Scalar, Orderable}  // a signed length of time

impl Eq for DateTime
impl Eq for Instant
impl Eq for Duration
impl Ord for DateTime
impl Ord for Instant
impl Ord for Duration
impl Add for Duration
impl Sub for Duration
```

```hql
// lib/std/time/mod.hql
module time
import core

type Zone : Orderable {
    name : String           // an IANA name, "Europe/Berlin", or a fixed offset, "+02:00"
}

abstract type Unit
type Nanosecond  : Unit
type Microsecond : Unit
type Millisecond : Unit
type Second      : Unit
type Minute      : Unit
type Hour        : Unit
type Day         : Unit
type Week        : Unit     // ISO: starts on Monday
type Month       : Unit
type Quarter     : Unit
type Year        : Unit
```

- An `Instant` MUST have nanosecond precision and MUST cover at least the
  years `-9999` to `9999`. A `Duration` MUST be a signed count of nanoseconds
  that holds at least the difference between those bounds.
- A `DateTime` MUST NOT carry a zone or an offset. It names a wall-clock
  reading, and becomes an instant only through a zone.
- `Utc` MUST be the zone `Zone("UTC")`, declared as a value in `time`.
- A `Zone` MUST be constructed from an IANA name the binary's database holds
  or from a fixed offset `±HH:MM`; any other text is an evaluation error.

## Construction and conversion

Every constructor reads ISO 8601 text or named parts. The constructors of the
`core` types are declared in `core`, beside their types, as
[HQL-0018](../../HQL-0018/README.md) places a constructor; the conversions and
`Zone`'s constructor are `time`'s.

```hql
// lib/std/core/mod.hql
fn Date(text: String) : Date                         // "2026-10-24"
fn Date(*, year: Int, month: Int, day: Int) : Date
fn DateTime(text: String) : DateTime                 // "2026-10-24T09:30:00"
fn DateTime(date: Date, *, hour: Int = 0, minute: Int = 0, second: Int = 0, nanosecond: Int = 0) : DateTime
fn Instant(text: String) : Instant                   // RFC 3339: "2026-10-24T07:30:00Z"
fn Instant(*, unix_seconds: Int) : Instant
fn Instant(*, unix_millis: Int) : Instant
fn Duration(text: String) : Duration                 // "PT1H30M", "P2DT3H"
fn Duration(*, days: Int = 0, hours: Int = 0, minutes: Int = 0, seconds: Int = 0, millis: Int = 0, nanos: Int = 0) : Duration

// lib/std/time/mod.hql
fn Zone(name: String) : Zone
fn to_date(self: String) : Option<Date>
fn to_datetime(self: String) : Option<DateTime>
fn to_instant(self: String) : Option<Instant>
fn to_duration(self: String) : Option<Duration>

fn parse_date(self: String, pattern: String) : Option<Date>
fn parse_datetime(self: String, pattern: String) : Option<DateTime>
fn parse_instant(self: String, pattern: String) : Option<Instant>
```

- A text constructor MUST accept exactly the ISO 8601 extended form named in
  its comment. `Instant(text)` MUST require an offset or `Z`; text without
  one is a `DateTime`, and reading it as an instant needs a zone.
- `Duration(text)` MUST reject the year and month designators (`P1Y`,
  `P1M`): their length depends on the date they start from. Calendar
  lengths are `add_months` and `add_years`.
- A named-part constructor MUST reject a part out of range: `month = 13`,
  `day = 30` in February, `hour = 24`.
- `parse_*` MUST read the strftime pattern language: `%Y %m %d %H %M %S %f
  %z %Z %j %a %A %b %B %p %%`, with `%z` an offset and `%Z` an IANA name. A
  pattern directive outside this set is an evaluation error at the call.
- The `to_*` and `parse_*` conversions MUST give `none` for text that does
  not parse; the constructors fail on the same text, as
  [no silent coercion](../README.md#3-no-silent-coercion) states.

## Zones

A zone is the one thing that turns a wall-clock reading into an instant and
back.

```hql
fn at(self: Instant, zone: Zone) : DateTime
fn in_zone(self: DateTime, zone: Zone) : Instant
fn offset(self: Zone, at: Instant) : Duration
```

- `in_zone` MUST resolve a reading that falls in a gap (clocks moved forward)
  by moving it forward by the gap's length, and a reading that falls in an
  overlap (clocks moved back) to the earlier of the two instants. This is
  RFC 9557's `compatible` disambiguation; one rule, never an error.
- `instant.at(zone).in_zone(zone)` MUST equal `instant`.

## Parts

```hql
fn year(self: Date) : Int
fn month(self: Date) : Int           // 1..12
fn day(self: Date) : Int             // 1..31
fn weekday(self: Date) : Int         // ISO: Monday is 1, Sunday is 7
fn day_of_year(self: Date) : Int     // 1..366
fn iso_week(self: Date) : Int        // 1..53
fn iso_year(self: Date) : Int        // the year iso_week belongs to

fn date(self: DateTime) : Date
fn hour(self: DateTime) : Int
fn minute(self: DateTime) : Int
fn second(self: DateTime) : Int
fn nanosecond(self: DateTime) : Int

fn unix_seconds(self: Instant) : Int
fn unix_millis(self: Instant) : Int
fn total_seconds(self: Duration) : Decimal
fn total_millis(self: Duration) : Int
```

- `year`, `month`, `day`, `weekday`, `day_of_year`, `iso_week` and
  `iso_year` MUST also be declared over `DateTime`, reading its date.
- `unix_seconds` and `unix_millis` MUST round toward negative infinity, so
  an instant before 1970 that is not on a whole second reads the second
  before it.
- `total_seconds` MUST be exact; `total_millis` MUST round toward zero.

## Arithmetic

An instant moves by a duration; a date and a date-time move by calendar
units.

```hql
fn plus(self: Instant, by: Duration) : Instant
fn minus(self: Instant, by: Duration) : Instant
fn since(self: Instant, earlier: Instant) : Duration

fn plus(self: DateTime, by: Duration) : DateTime
fn minus(self: DateTime, by: Duration) : DateTime
fn since(self: DateTime, earlier: DateTime) : Duration

fn add_days(self: Date, n: Int) : Date
fn add_months(self: Date, n: Int) : Date
fn add_years(self: Date, n: Int) : Date
fn days_since(self: Date, earlier: Date) : Int

fn negate(self: Duration) : Duration
fn abs(self: Duration) : Duration
fn times(self: Duration, n: Int) : Duration
```

- `add_days`, `add_months` and `add_years` MUST also be declared over
  `DateTime`, moving its date and keeping its time of day.
- `add_months` and `add_years` MUST clamp to the last day of the resulting
  month: `Date("2026-01-31").add_months(1)` is `2026-02-28`.
- `DateTime` arithmetic MUST be wall-clock arithmetic, independent of any
  zone: `DateTime("2026-03-29T01:30:00").plus(Duration(hours = 1))` is
  `02:30`, whether or not some zone skips that hour.
- `+` and `-` MUST be defined on `Duration` through `Add` and `Sub`, and on
  no other time type.
- Arithmetic that leaves the representable range MUST be an evaluation error.

## Truncation and ranges

Grouping by time is truncating to a unit.

```hql
fn truncate<U : Unit>(self: Instant, unit: Type<U>, zone: Zone = Utc) : Instant
fn truncate<U : Unit>(self: DateTime, unit: Type<U>) : DateTime
fn truncate<U : Unit>(self: Date, unit: Type<U>) : Date
fn dates(start: Date, end: Date) : List<Date>
```

- `truncate` MUST give the start of the unit containing the value: the
  Monday of its ISO week for `Week`, the first day of January, April, July
  or October for `Quarter`.
- `truncate` over an `Instant` MUST truncate the instant's reading in `zone`
  and convert the result back through the same zone, so `Day` in
  `Europe/Berlin` is Berlin's midnight.
- `truncate` over a `Date` with a unit below `Day` MUST return the date
  unchanged.
- `dates(start, end)` MUST give every date from `start` up to and excluding
  `end`, as `range` does over `Int`, and the empty list when `end` is not
  after `start`.

## Rendering

```hql
fn fmt(self: Date, pattern: String) : String
fn fmt(self: DateTime, pattern: String) : String
fn fmt(self: Instant, pattern: String, zone: Zone = Utc) : String
fn str(self: Duration) : String
```

- `fmt` MUST read the same pattern language as `parse_*`, and
  `x.fmt(p).parse_date(p)` MUST give `some(x)` for a pattern that writes
  every part of `x`.
- `str` of each time value MUST be its ISO 8601 text, the text its
  constructor reads.

## The clock

```hql
effectful fn now() : Instant
effectful fn today(zone: Zone) : Date
effectful fn today() : Date
effectful fn local_zone() : Zone
```

- `now` MUST return the evaluation instant, fixed once per evaluation as
  [values fixed per evaluation](../README.md#2-values-fixed-per-evaluation)
  states.
- `local_zone` MUST return the zone named by `--zone`, else by the `TZ`
  environment variable, else the host's, else `Utc`. It is read once per
  evaluation.
- `today()` MUST equal `today(local_zone())`, and `today(z)` MUST equal
  `now().at(z).date()`.

## Examples

```hql
import time

// Cards whose metadata carries a date, newest first, grouped by month.
vault.cards
| map(c => {card: c, date: c.metadata["date"] | and_then(d => str(d).to_date())})
| filter(r => r.date.is_some())
| group(by = r => r.date | unwrap_or(Date("1970-01-01")) | truncate(Month))
| sort(by = g => g.key)
| reverse

// The age of each sighting, in whole days.
source(file("sightings.jsonl"), JsonLines)
| map(r => today().days_since(r.seen.to_date() | unwrap_or(today())))

// A Berlin wall-clock meeting as an instant, and the hours until it.
DateTime("2026-11-02T10:00:00").in_zone(Zone("Europe/Berlin")).since(now())
| total_seconds / 3600
```

## Test plan

- Construction: each text form, each rejected form (an offset-less
  `Instant`, `P1M`, `month = 13`, February 30), and the `to_*` conversions
  over the same text.
- Zones: a gap and an overlap in `Europe/Berlin` resolved as stated; the
  round trip through `at` and `in_zone`; a fixed offset; an unknown name.
- Calendar: `add_months` clamping at month ends and across a leap year,
  `iso_week` at year boundaries (`2026-12-31`, `2027-01-01`), `weekday` of a
  known date.
- Truncation: every unit over a date, a date-time and an instant in a zone
  with a DST change inside the unit.
- Rendering: `fmt` and `parse_*` round trips for every directive; `str` and
  the constructor round trip for each type.
- The clock: `now()` twice in one evaluation is equal; `--now` and `--zone`
  fix `now()`, `today()` and `local_zone()`.
