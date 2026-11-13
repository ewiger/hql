// Every declaration is external: the `std` extension implements it.
//
// See doc/proposals/HQL-0041/parts/01-time.hmd.

/// Time: zones, the units a value truncates to, and the operations over the
/// time scalars of `core`: conversion from text, parts, arithmetic,
/// truncation, rendering, and the evaluation clock. Not in the prelude: a
/// program writes `import time`.
module time

import core

/// A time zone: an IANA name the time-zone database holds,
/// `"Europe/Berlin"`, `"UTC"`, or a fixed offset `"+02:00"`. A zone turns
/// a wall-clock `DateTime` into an `Instant` and back. The database is the
/// host's where it has one, else the one compiled into the binary, and
/// `hql --version` names which: `system`, or the bundled release.
type Zone {
    name : String
}

/// The units a time value truncates to.
abstract type TimeUnit
type Nanosecond  : TimeUnit
type Microsecond : TimeUnit
type Millisecond : TimeUnit
type Second      : TimeUnit
type Minute      : TimeUnit
type Hour        : TimeUnit
type Day         : TimeUnit
type Week        : TimeUnit   // an ISO week, starting on Monday
type Month       : TimeUnit
type Quarter     : TimeUnit   // from January, April, July or October
type Year        : TimeUnit

// Construction and conversion.

/// The zone `name` names: an IANA name, `"UTC"`, or an offset `"±HH:MM"`.
/// Any other text is an evaluation error naming it.
fn Zone(name: String) : Zone

/// `self` read as an ISO 8601 date, `some(Date)`, or `none` where it is not
/// one: `"2026-10-24".to_date()` is `some(2026-10-24)`.
fn to_date(self: String) : Option<Date>

/// `self` read as an ISO 8601 date-time with no offset, or `none`.
fn to_datetime(self: String) : Option<DateTime>

/// `self` read as an RFC 3339 instant, with `Z` or an offset, or `none`.
fn to_instant(self: String) : Option<Instant>

/// `self` read as an ISO 8601 duration without years or months, or `none`.
fn to_duration(self: String) : Option<Duration>

/// `self` read through a strftime `pattern` as a `Date`, or `none` where it
/// does not match: `"24.10.2026".parse_date("%d.%m.%Y")`. The directives
/// are `%Y %m %d %H %M %S %f %z %Z %j %a %A %b %B %p %%`; any other is an
/// evaluation error.
fn parse_date(self: String, pattern: String) : Option<Date>

/// `self` read as an ISO 8601 date, or `none`: with no pattern,
/// `parse_date` is `to_date`.
fn parse_date(self: String) : Option<Date>

/// `self` read through a strftime `pattern` as a `DateTime`, or `none`.
fn parse_datetime(self: String, pattern: String) : Option<DateTime>

/// `self` read as an ISO 8601 date-time with no offset, or `none`: with no
/// pattern, `parse_datetime` is `to_datetime`.
fn parse_datetime(self: String) : Option<DateTime>

/// `self` read through a strftime `pattern` as an `Instant`, or `none`; the
/// pattern writes an offset with `%z` or a zone with `%Z`.
fn parse_instant(self: String, pattern: String) : Option<Instant>

/// `self` read as an RFC 3339 instant, or `none`: with no pattern,
/// `parse_instant` is `to_instant`.
fn parse_instant(self: String) : Option<Instant>

// Zones.

/// The wall-clock reading of `self` in `zone`.
fn at(self: Instant, zone: Zone) : DateTime

/// The instant the wall-clock reading `self` names in `zone`. A reading in
/// a gap moves forward by the gap's length, and one in an overlap is the
/// earlier of its two instants.
fn in_zone(self: DateTime, zone: Zone) : Instant

/// The offset from UTC `self` has at the instant `at`: `PT2H` in Berlin
/// in summer.
fn offset(self: Zone, at: Instant) : Duration

// Parts.

/// The year of `self`.
fn year(self: Date) : Int
fn year(self: DateTime) : Int

/// The month of `self`, `1..12`.
fn month(self: Date) : Int
fn month(self: DateTime) : Int

/// The day of the month of `self`, `1..31`.
fn day(self: Date) : Int
fn day(self: DateTime) : Int

/// The ISO day of the week of `self`: Monday is 1, Sunday is 7.
fn weekday(self: Date) : Int
fn weekday(self: DateTime) : Int

/// The day of the year of `self`, `1..366`.
fn day_of_year(self: Date) : Int
fn day_of_year(self: DateTime) : Int

/// The ISO week of `self`, `1..53`, in the year `iso_year` gives.
fn iso_week(self: Date) : Int
fn iso_week(self: DateTime) : Int

/// The year the ISO week of `self` belongs to: `2025` for `2024-12-30`,
/// the Monday of the first ISO week of 2025.
fn iso_year(self: Date) : Int
fn iso_year(self: DateTime) : Int

/// The date of `self`.
fn date(self: DateTime) : Date

/// The hour of `self`, `0..23`.
fn hour(self: DateTime) : Int

/// The minute of `self`, `0..59`.
fn minute(self: DateTime) : Int

/// The second of `self`, `0..59`.
fn second(self: DateTime) : Int

/// The nanoseconds of `self` past its second.
fn nanosecond(self: DateTime) : Int

/// The whole seconds from `1970-01-01T00:00:00Z` to `self`, rounded toward
/// negative infinity.
fn unix_seconds(self: Instant) : Int

/// The whole milliseconds from `1970-01-01T00:00:00Z` to `self`, rounded
/// toward negative infinity.
fn unix_millis(self: Instant) : Int

/// The length of `self` in seconds, exactly: `PT1.5S` is `1.5`.
fn total_seconds(self: Duration) : Decimal

/// The length of `self` in whole milliseconds, rounded toward zero.
fn total_millis(self: Duration) : Int

// Arithmetic.

/// `self` moved later by `by`, earlier where `by` is negative.
fn plus(self: Instant, by: Duration) : Instant
fn plus(self: DateTime, by: Duration) : DateTime

/// `self` moved earlier by `by`.
fn minus(self: Instant, by: Duration) : Instant
fn minus(self: DateTime, by: Duration) : DateTime

/// The duration from `earlier` to `self`, negative where `earlier` is after
/// `self`. Over `DateTime` it is wall-clock time, independent of any zone.
fn since(self: Instant, earlier: Instant) : Duration
fn since(self: DateTime, earlier: DateTime) : Duration

/// `self` moved by `n` days.
fn add_days(self: Date, n: Int) : Date
fn add_days(self: DateTime, n: Int) : DateTime

/// `self` moved by `n` months, clamped to the last day of the month it
/// lands in: `Date("2026-01-31").add_months(1)` is `2026-02-28`.
fn add_months(self: Date, n: Int) : Date
fn add_months(self: DateTime, n: Int) : DateTime

/// `self` moved by `n` years, clamped as `add_months` is: February 29
/// moved by a year is February 28.
fn add_years(self: Date, n: Int) : Date
fn add_years(self: DateTime, n: Int) : DateTime

/// The days from `earlier` to `self`, negative where `earlier` is after it.
fn days_since(self: Date, earlier: Date) : Int

/// `self` with its sign reversed.
fn negate(self: Duration) : Duration

/// `self` multiplied by `n`.
fn times(self: Duration, n: Int) : Duration

// Truncation and ranges.

/// The start of the `unit` containing `self`: the first day of the month
/// for `Month`, the Monday of its ISO week for `Week`. A unit below `Day`
/// leaves a `Date` unchanged.
fn truncate<U : TimeUnit>(self: Date, unit: Type<U>) : Date
fn truncate<U : TimeUnit>(self: DateTime, unit: Type<U>) : DateTime

/// The start of the `unit` containing `self`, read in UTC, or in `zone`
/// where one is given: `Day` in `Zone("Europe/Berlin")` is Berlin's
/// midnight.
fn truncate<U : TimeUnit>(self: Instant, unit: Type<U>) : Instant
fn truncate<U : TimeUnit>(self: Instant, unit: Type<U>, zone: Zone) : Instant

/// Every date from `self` up to and excluding `end`, as `range` gives the
/// `Int`s; empty where `end` is not after `self`.
fn dates(self: Date, end: Date) : List<Date>

// Rendering. `str` of a time value is its ISO 8601 text.

/// `self` written through a strftime `pattern`, the directives `parse_date`
/// reads: `Date("2026-10-24").fmt("%d %B %Y")` is `"24 October 2026"`.
fn fmt(self: Date, pattern: String) : String
fn fmt(self: DateTime, pattern: String) : String

/// `self` written through a strftime `pattern` as its reading in UTC, or in
/// `zone` where one is given.
fn fmt(self: Instant, pattern: String) : String
fn fmt(self: Instant, pattern: String, zone: Zone) : String

// The clock: read once per evaluation, so two reads agree. `--now` and
// `--zone` fix it.

/// The evaluation instant: the host's clock, or `--now`.
effectful fn now() : Instant

/// The date `now()` falls on in `zone`, or in `local_zone()`.
effectful fn today() : Date
effectful fn today(zone: Zone) : Date

/// The evaluation zone: `--zone`, else the `TZ` environment variable, else
/// the host's, else `UTC`.
effectful fn local_zone() : Zone
