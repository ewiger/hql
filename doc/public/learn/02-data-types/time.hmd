# Date, DateTime, Instant and Duration

HQL has four time types. None of them has a literal: a value is built by the
type's constructor from ISO 8601 text, the form SQL and JSON write it in.

| Type | What it holds | Written as |
| --- | --- | --- |
| `Date` | a calendar date | `2026-10-24` |
| `DateTime` | a date and a time of day, with no zone | `2026-10-24T09:30:00` |
| `Instant` | a point on the UTC timeline, to the nanosecond | `2026-10-24T07:30:00Z` |
| `Duration` | a signed length of time, to the nanosecond | `PT1H30M` |

```hql
[Date("2026-10-24"), DateTime("2026-10-24T09:30:00"), Instant("2026-10-24T09:30:00+02:00"), Duration("PT1H30M")]
```

```text
[2026-10-24, 2026-10-24T09:30:00, 2026-10-24T07:30:00Z, PT1H30M]
```

An `Instant` is read with its offset or `Z` and written in UTC, so
`09:30+02:00` is `07:30Z`. A `DateTime` names a wall-clock reading and
carries no offset; it becomes an `Instant` only through a zone. A `Duration`
counts days as 24 hours and has no months or years, whose length depends on
the date they start from.

A constructor also takes named parts: `Date(year =, month =, day =)`,
`DateTime(date, hour =, minute =)`, `Instant(unix_seconds =)` and
`Duration(hours =, minutes =, ...)`. Text that does not parse, or a part out
of range, is an evaluation error naming it:

```hql
Date(year = 2026, month = 2, day = 30)
```

```text
execute failure[invalid-argument]: year 2026, month 2, day 30 is not a date
 --> <expression>:1:1
  |
1 | Date(year = 2026, month = 2, day = 30)
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## Order and equality

Each time type is `Orderable`: `==`, `<`, `sort`, `min` and `max` compare
two dates, two date-times or two instants by time, and two durations by
length. Two instants written with different offsets are equal when they name
one point:

```hql
[Date("2026-10-24"), Date("2025-12-31"), Date("2026-01-01")] | sort
```

```text
[2025-12-31, 2026-01-01, 2026-10-24]
```

`+` and `-` take two `Duration`s, and no other time type. An instant or a
date moves by a named operation of the `time` module:

```hql
Duration("PT1H") + Duration(minutes = 30) > Duration("PT1H15M")
```

```text
true
```

`abs`, which the prelude's `math` module declares over numbers, also takes a
`Duration` and gives its length without its sign, so the gap between two
times reads the same whichever is subtracted from which:

```hql
[abs(Duration("PT1H") - Duration("PT90M")), Duration("PT2H").abs()]
```

```text
[PT30M, PT2H]
```

## The time module

Every operation over a time value is in the module `time`, which is not in
the prelude: a program writes `import time`.

### Parts and calendar arithmetic

`year`, `month`, `day`, `weekday` (Monday is 1) and `iso_week` read a date's
parts, and `hour`, `minute` and `second` a date-time's:

```hql
import time
d = Date("2026-10-24")
[d.year(), d.month(), d.day(), d.weekday(), d.iso_week()]
```

```text
[2026, 10, 24, 6, 43]
```

`add_days`, `add_months` and `add_years` move a date by calendar units.
A month added to January 31 lands on the last day of February. `truncate`
gives the start of the `Week`, `Month`, `Quarter` or `Year` containing a
date, the key a grouping by period uses:

```hql
import time
[Date("2026-01-31").add_months(1), Date("2026-10-24").add_days(10), Date("2026-11-12").truncate(Month)]
```

```text
[2026-02-28, 2026-11-03, 2026-11-01]
```

`days_since` counts the days between two dates, `since` gives the
`Duration` between two instants, and `plus` and `minus` move an instant by
one:

```hql
import time
Instant("2026-10-24T07:30:00Z").since(Instant("2026-10-23T22:00:00Z"))
```

```text
PT9H30M
```

### Zones

`Zone(name)` names a zone: an IANA name such as `"Europe/Berlin"`, `"UTC"`,
or a fixed offset `"+05:30"`. `reading.in_zone(zone)` gives the instant a
wall-clock reading names there, and `instant.at(zone)` the reading an
instant has there:

```hql
import time
berlin = Zone("Europe/Berlin")
meeting = DateTime("2026-11-02T10:00:00").in_zone(berlin)
[str(meeting), str(meeting.at(Zone("America/New_York")))]
```

```text
["2026-11-02T09:00:00Z", "2026-11-02T04:00:00"]
```

When clocks move forward, a reading inside the skipped hour moves forward by
the gap; when they move back, a reading that happens twice is the earlier
instant. On 2026-03-29 Berlin skips 02:00 to 03:00, and on 2026-10-25 it
repeats 02:00 to 03:00:

```hql
import time
[DateTime("2026-03-29T02:30:00").in_zone(Zone("Europe/Berlin")), DateTime("2026-10-25T02:30:00").in_zone(Zone("Europe/Berlin"))]
```

```text
[2026-03-29T01:30:00Z, 2026-10-25T00:30:00Z]
```

A zone's rules come from a time-zone database: the host's where it has one,
else the one compiled into `hql`. A reading in the future can move when that
database changes its rules, so `hql --version` names the database on its
second line: `system` for the host's, or the bundled database's release,
`2026e, bundled`:

```text
$ hql --version
hql 0.6.0
time-zone database: system
```

### Text in other forms

`fmt(pattern)` writes a value through a strftime pattern, and `parse_date`,
`parse_datetime` and `parse_instant` read text through one. The directives
are `%Y %m %d %H %M %S %f %z %Z %j %a %A %b %B %p %%`:

```hql
import time
[Date("2026-10-24").fmt("%A, %d %B %Y"), str("24.10.2026".parse_date("%d.%m.%Y"))]
```

```text
["Saturday, 24 October 2026", "2026-10-24"]
```

`to_date`, `to_datetime`, `to_instant` and `to_duration` read ISO 8601 text
as an `Option`: the value where the text is one, and `none` where it is not.
A parse function gives `none` the same way:

```hql
import time
["2026-10-24".to_date(), "24 Oct".to_date()]
```

```text
[2026-10-24, none]
```

A parse function given no pattern reads what its `to_*` reads:
`s.parse_date()` is `s.to_date()`, and the same for `parse_datetime` and
`parse_instant`:

```hql
import time
["1982-05-09".parse_date(), "09.05.1982".parse_date()]
```

```text
[1982-05-09, none]
```

### The evaluation clock

`now()` gives the evaluation instant, `today()` the date it falls on in the
evaluation zone, and `local_zone()` that zone. Each is read once per
evaluation, so two calls in one program agree. `--now` and `--zone` fix them,
which makes a program that reads the clock repeatable:

<!-- run: --now 2026-10-24T23:30:00Z --zone Europe/Berlin -->
```hql
import time
[str(now()), str(today()), local_zone().name]
```

```text
["2026-10-24T23:30:00Z", "2026-10-25", "Europe/Berlin"]
```

Without `--zone`, the zone is the one `TZ` names, else the host's.

## Time in data

Every format writes a time value as its ISO 8601 text, a JSON string:

<!-- run: --to json -->
```hql
{on: Date("2026-10-24"), took: Duration("PT1H30M")}
```

```text
{
  "on": "2026-10-24",
  "took": "PT1H30M"
}
```

Reading is the other way round only where a type says so. A JSON or YAML
string that looks like a date stays a `String`; a field declared `Date`,
`DateTime`, `Instant` or `Duration` reads its text as one, and text that is
not one fails the read:

```hql
type Event {
    name : String
    on   : Date
}
Data("{\"name\": \"launch\", \"on\": \"2026-10-24\"}", Json) | Event
```

```text
Event(name = "launch", on = 2026-10-24)
```

SQL tables read the same way. A PostgreSQL `DATE`, `TIMESTAMP` and
`TIMESTAMPTZ` column reads as a `Date`, a `DateTime` and an `Instant`, and
Spark's `DATE`, `TIMESTAMP_NTZ` and `TIMESTAMP` likewise; a field of a time
type written to SQLite, or to any text column, is its ISO 8601 text.
