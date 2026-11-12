//! The time scalars of `core` (HQL-0041 part 1): `Date`, `DateTime`,
//! `Instant` and `Duration`, their ISO 8601 text, and the calendar, zone and
//! pattern operations the `time` module runs over them.
//!
//! The values are `jiff`'s: a proleptic Gregorian date, a wall-clock
//! date-time with no zone, a nanosecond timestamp on the UTC timeline, and a
//! signed nanosecond duration. Everything here is pure: the evaluation
//! clock and zone are [`Clock`], fixed once per evaluation by the host.

use jiff::civil;
use jiff::tz::{Offset, TimeZone};
use jiff::{SignedDuration, Timestamp};
use std::fmt;
use std::sync::{Mutex, PoisonError};

/// A value of one of the four time scalars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Time {
    /// A calendar date: `2026-10-24`.
    Date(civil::Date),
    /// A date and a time of day, with no zone: `2026-10-24T09:30:00`.
    DateTime(civil::DateTime),
    /// A point on the UTC timeline: `2026-10-24T07:30:00Z`.
    Instant(Timestamp),
    /// A signed length of time: `PT1H30M`.
    Duration(SignedDuration),
}

impl Time {
    /// The name of the value's type, as `core` declares it.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Date(_) => "Date",
            Self::DateTime(_) => "DateTime",
            Self::Instant(_) => "Instant",
            Self::Duration(_) => "Duration",
        }
    }

    /// The order of two values of one type; values of different types order
    /// by type, so a mixed sort is deterministic.
    #[must_use]
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Date(a), Self::Date(b)) => a.cmp(b),
            (Self::DateTime(a), Self::DateTime(b)) => a.cmp(b),
            (Self::Instant(a), Self::Instant(b)) => a.cmp(b),
            (Self::Duration(a), Self::Duration(b)) => a.cmp(b),
            _ => self.rank().cmp(&other.rank()),
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Self::Date(_) => 0,
            Self::DateTime(_) => 1,
            Self::Instant(_) => 2,
            Self::Duration(_) => 3,
        }
    }

    /// The value of a type's name read from its ISO 8601 text, as its
    /// constructor reads it.
    #[must_use]
    pub fn parse_as(type_name: &str, text: &str) -> Option<Self> {
        match type_name {
            "Date" => parse_date(text).map(Self::Date),
            "DateTime" => parse_datetime(text).map(Self::DateTime),
            "Instant" => parse_instant(text).map(Self::Instant),
            "Duration" => parse_duration(text).map(Self::Duration),
            _ => None,
        }
    }
}

impl Time {
    /// The value of a type's name read from text a data source wrote: the
    /// ISO 8601 text its constructor reads, or the SQL form with a space
    /// for the `T` and an offset of whole hours, `2026-10-24 09:30:00+02`,
    /// as PostgreSQL and Spark write a timestamp. An `Instant` in the SQL
    /// form with no offset is read in UTC, the zone Spark's sessions write
    /// a `TIMESTAMP` in unless configured otherwise.
    #[must_use]
    pub fn read_as(type_name: &str, text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some(read) = Self::parse_as(type_name, text) {
            return Some(read);
        }
        let (date, time) = text.split_once(' ')?;
        let mut joined = format!("{date}T{time}");
        if type_name == "Instant" {
            let offset_at = joined.rfind(['+', '-']).filter(|at| *at > date.len());
            match offset_at {
                Some(at) if joined.len() - at == 3 => joined.push_str(":00"),
                None if !joined.ends_with(['Z', 'z']) => joined.push('Z'),
                _ => {}
            }
        }
        Self::parse_as(type_name, &joined)
    }
}

impl fmt::Display for Time {
    /// The ISO 8601 text, the text the type's constructor reads: RFC 3339
    /// for an `Instant`, and a `Duration` in hours, minutes and seconds.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Date(date) => write!(f, "{date}"),
            Self::DateTime(datetime) => write!(f, "{datetime}"),
            Self::Instant(instant) => write!(f, "{instant}"),
            Self::Duration(duration) => f.write_str(&duration_text(*duration)),
        }
    }
}

/// Whether `text` is all ASCII digits and not empty.
fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// The date part's shape: `YYYY-MM-DD`, a six-digit year signed.
fn date_shaped(text: &str) -> bool {
    let body = text
        .strip_prefix(['+', '-'])
        .map_or(Some(text), |rest| (rest.len() == 15).then_some(rest));
    let Some(body) = body else { return false };
    let parts: Vec<&str> = body.split('-').collect();
    matches!(parts.as_slice(), [y, m, d]
        if (y.len() == 4 || (y.len() == 6 && text.len() != body.len()))
            && digits(y) && m.len() == 2 && digits(m) && d.len() == 2 && digits(d))
}

/// A date in the ISO 8601 extended form `2026-10-24`, and nothing else.
#[must_use]
pub fn parse_date(text: &str) -> Option<civil::Date> {
    date_shaped(text).then(|| text.parse().ok()).flatten()
}

/// The time part's shape: `HH:MM:SS`, with an optional fraction of up to
/// nine digits.
fn time_shaped(text: &str) -> bool {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, "0"));
    let parts: Vec<&str> = whole.split(':').collect();
    fraction.len() <= 9
        && digits(fraction)
        && matches!(parts.as_slice(), [h, m, s]
            if [h, m, s].iter().all(|part| part.len() == 2 && digits(part)))
}

/// A date-time in the ISO 8601 extended form `2026-10-24T09:30:00`, with no
/// offset or zone.
#[must_use]
pub fn parse_datetime(text: &str) -> Option<civil::DateTime> {
    let (date, time) = text.split_once('T')?;
    (date_shaped(date) && time_shaped(time))
        .then(|| text.parse().ok())
        .flatten()
}

/// An instant in RFC 3339: a date-time with `Z` or a numeric offset.
#[must_use]
pub fn parse_instant(text: &str) -> Option<Timestamp> {
    let (date, rest) = text.split_once(['T', 't'])?;
    let time_end = rest.find(['Z', 'z', '+', '-'])?;
    let (time, offset) = rest.split_at(time_end);
    let offset_shaped = matches!(offset, "Z" | "z")
        || (offset.len() == 6
            && offset.as_bytes()[3] == b':'
            && digits(&offset[1..3])
            && digits(&offset[4..]));
    (date_shaped(date) && time_shaped(time) && offset_shaped)
        .then(|| text.parse().ok())
        .flatten()
}

const NANOS_PER_SECOND: i128 = 1_000_000_000;

/// A duration from a count of nanoseconds, or `None` past its range.
#[must_use]
pub fn duration_of_nanos(nanos: i128) -> Option<SignedDuration> {
    SignedDuration::try_from_nanos_i128(nanos)
}

/// A duration in the ISO 8601 form `PnWnDTnHnMn.fS`, signed with a leading
/// `-`. The year and month designators are refused: their length depends on
/// the date they start from. A day is 24 hours and a week seven days.
#[must_use]
pub fn parse_duration(text: &str) -> Option<SignedDuration> {
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let body = body.strip_prefix(['P', 'p'])?;
    let (date, time) = match body.split_once(['T', 't']) {
        Some((date, time)) if !time.is_empty() => (date, Some(time)),
        Some(_) => return None,
        None => (body, None),
    };
    if date.is_empty() && time.is_none() {
        return None;
    }
    let mut nanos: i128 = 0;
    let mut add = |count: &str, unit: i128| -> Option<()> {
        nanos = nanos.checked_add(count.parse::<i128>().ok()?.checked_mul(unit)?)?;
        Some(())
    };
    let mut designators = |part: &str, units: &[(char, i128)], fraction_last: bool| {
        let mut rest = part;
        let mut allowed = units;
        while !rest.is_empty() {
            let at = rest.find(|c: char| !c.is_ascii_digit() && c != '.')?;
            let (number, tail) = rest.split_at(at);
            let designator = tail.chars().next()?.to_ascii_uppercase();
            let position = allowed.iter().position(|(d, _)| *d == designator)?;
            let unit = allowed[position].1;
            if number.contains('.') {
                // Only seconds take a fraction, as the last designator.
                if !fraction_last || designator != 'S' {
                    return None;
                }
                let (whole, fraction) = number.split_once('.')?;
                if !digits(whole) || !digits(fraction) || fraction.len() > 9 {
                    return None;
                }
                add(whole, unit)?;
                let scaled = format!("{fraction:0<9}");
                add(&scaled, 1)?;
            } else {
                if !digits(number) {
                    return None;
                }
                add(number, unit)?;
            }
            allowed = &allowed[position + 1..];
            rest = &tail[1..];
        }
        Some(())
    };
    let day = 86_400 * NANOS_PER_SECOND;
    designators(date, &[('W', 7 * day), ('D', day)], false)?;
    if let Some(time) = time {
        designators(
            time,
            &[
                ('H', 3_600 * NANOS_PER_SECOND),
                ('M', 60 * NANOS_PER_SECOND),
                ('S', NANOS_PER_SECOND),
            ],
            true,
        )?;
    }
    duration_of_nanos(if negative { -nanos } else { nanos })
}

/// A duration's ISO 8601 text in hours, minutes and seconds, `PT0S` for
/// zero: the text [`parse_duration`] reads back.
#[must_use]
pub fn duration_text(duration: SignedDuration) -> String {
    let nanos = duration.as_nanos();
    let sign = if nanos < 0 { "-" } else { "" };
    let total = nanos.unsigned_abs();
    let seconds = total / 1_000_000_000;
    let fraction = total % 1_000_000_000;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    let mut out = format!("{sign}PT");
    if hours > 0 {
        out.push_str(&format!("{hours}H"));
    }
    if minutes > 0 {
        out.push_str(&format!("{minutes}M"));
    }
    if seconds > 0 || fraction > 0 || (hours == 0 && minutes == 0) {
        out.push_str(&seconds.to_string());
        if fraction > 0 {
            let digits = format!("{fraction:09}");
            out.push('.');
            out.push_str(digits.trim_end_matches('0'));
        }
        out.push('S');
    }
    out
}

/// A zone by its name: an IANA name the compiled database holds, `UTC`, or
/// a fixed offset `±HH:MM`.
///
/// # Errors
/// Names text that is neither.
pub fn zone(name: &str) -> Result<TimeZone, String> {
    if let Some(offset) = fixed_offset(name) {
        return Ok(TimeZone::fixed(offset));
    }
    TimeZone::get(name).map_err(|_| {
        format!("{name:?} is not a zone: write an IANA name such as \"Europe/Berlin\", or an offset such as \"+02:00\"")
    })
}

/// A fixed offset written `±HH:MM`.
fn fixed_offset(text: &str) -> Option<Offset> {
    let sign = match text.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let rest = &text[1..];
    let (hours, minutes) = rest.split_once(':')?;
    if hours.len() != 2 || minutes.len() != 2 || !digits(hours) || !digits(minutes) {
        return None;
    }
    let (hours, minutes): (i32, i32) = (hours.parse().ok()?, minutes.parse().ok()?);
    if minutes >= 60 {
        return None;
    }
    Offset::from_seconds(sign * (hours * 3600 + minutes * 60)).ok()
}

/// The units a value truncates to, finest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unit {
    /// A nanosecond.
    Nanosecond,
    /// A microsecond.
    Microsecond,
    /// A millisecond.
    Millisecond,
    /// A second.
    Second,
    /// A minute.
    Minute,
    /// An hour.
    Hour,
    /// A day.
    Day,
    /// An ISO week, from Monday.
    Week,
    /// A month.
    Month,
    /// A quarter, from January, April, July or October.
    Quarter,
    /// A year.
    Year,
}

impl Unit {
    /// The unit a type of `time` names: `Month` for `Month`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "Nanosecond" => Self::Nanosecond,
            "Microsecond" => Self::Microsecond,
            "Millisecond" => Self::Millisecond,
            "Second" => Self::Second,
            "Minute" => Self::Minute,
            "Hour" => Self::Hour,
            "Day" => Self::Day,
            "Week" => Self::Week,
            "Month" => Self::Month,
            "Quarter" => Self::Quarter,
            "Year" => Self::Year,
            _ => return None,
        })
    }
}

/// The first day of the unit containing `date`; a unit below a day leaves
/// it unchanged.
#[must_use]
pub fn truncate_date(date: civil::Date, unit: Unit) -> civil::Date {
    match unit {
        Unit::Week => {
            let back = i64::from(date.weekday().to_monday_one_offset()) - 1;
            date.checked_sub(jiff::Span::new().days(back))
                .unwrap_or(date)
        }
        Unit::Month => date.first_of_month(),
        Unit::Quarter => {
            let month = (date.month() - 1) / 3 * 3 + 1;
            civil::Date::new(date.year(), month, 1).unwrap_or(date)
        }
        Unit::Year => date.first_of_year(),
        _ => date,
    }
}

/// The start of the unit containing `datetime`, as a wall-clock reading.
#[must_use]
pub fn truncate_datetime(datetime: civil::DateTime, unit: Unit) -> civil::DateTime {
    let time = datetime.time();
    let (h, m, s, n) = (
        time.hour(),
        time.minute(),
        time.second(),
        time.subsec_nanosecond(),
    );
    let kept = |h, m, s, n| {
        civil::DateTime::from_parts(
            datetime.date(),
            civil::Time::new(h, m, s, n).unwrap_or(time),
        )
    };
    match unit {
        Unit::Nanosecond => datetime,
        Unit::Microsecond => kept(h, m, s, n / 1_000 * 1_000),
        Unit::Millisecond => kept(h, m, s, n / 1_000_000 * 1_000_000),
        Unit::Second => kept(h, m, s, 0),
        Unit::Minute => kept(h, m, 0, 0),
        Unit::Hour => kept(h, 0, 0, 0),
        Unit::Day | Unit::Week | Unit::Month | Unit::Quarter | Unit::Year => {
            truncate_date(datetime.date(), unit).to_datetime(civil::Time::midnight())
        }
    }
}

/// A wall-clock reading in `zone` as an instant: a reading in a gap moves
/// forward by the gap, and one in an overlap takes the earlier instant
/// (RFC 9557's `compatible`).
///
/// # Errors
/// A reading outside the representable range.
pub fn in_zone(datetime: civil::DateTime, zone: &TimeZone) -> Result<Timestamp, String> {
    zone.to_ambiguous_timestamp(datetime)
        .compatible()
        .map_err(|error| error.to_string())
}

/// The directives `fmt` and `parse_*` read; any other is refused.
const DIRECTIVES: &[char] = &[
    'Y', 'm', 'd', 'H', 'M', 'S', 'f', 'z', 'Z', 'j', 'a', 'A', 'b', 'B', 'p', '%',
];

/// Refuse a pattern that writes a directive outside the supported set.
///
/// # Errors
/// Names the first unsupported directive.
pub fn checked_pattern(pattern: &str) -> Result<(), String> {
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some(d) if DIRECTIVES.contains(&d) => {}
                Some(d) => {
                    return Err(format!(
                        "`%{d}` is not a pattern directive: write one of %Y %m %d %H %M %S %f %z %Z %j %a %A %b %B %p %%"
                    ));
                }
                None => return Err("a pattern ends with a lone `%`".to_owned()),
            }
        }
    }
    Ok(())
}

/// A pattern with `%f` read as the nine-digit fraction jiff writes `%N`.
fn jiff_pattern(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('f') => out.push_str("%N"),
                Some('Z') => out.push_str("%Q"),
                Some(d) => {
                    out.push('%');
                    out.push(d);
                }
                None => out.push('%'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A value written in a strftime pattern: a date or date-time as its own
/// reading, an instant as its reading in `zone`.
///
/// # Errors
/// An unsupported directive, or one the value has no part for.
pub fn format(value: &Time, pattern: &str, zone: &TimeZone) -> Result<String, String> {
    checked_pattern(pattern)?;
    let pattern = jiff_pattern(pattern);
    let written = match value {
        Time::Date(date) => jiff::fmt::strtime::format(&pattern, *date),
        Time::DateTime(datetime) => jiff::fmt::strtime::format(&pattern, *datetime),
        Time::Instant(instant) => {
            jiff::fmt::strtime::format(&pattern, &instant.to_zoned(zone.clone()))
        }
        Time::Duration(_) => return Err("a Duration has no pattern; use `str`".to_owned()),
    };
    written.map_err(|error| error.to_string())
}

/// Text read through a strftime pattern as the type named, or `None` when
/// it does not parse.
///
/// # Errors
/// An unsupported directive in the pattern.
pub fn parse_pattern(type_name: &str, text: &str, pattern: &str) -> Result<Option<Time>, String> {
    checked_pattern(pattern)?;
    let pattern = jiff_pattern(pattern);
    let Ok(parsed) = jiff::fmt::strtime::parse(&pattern, text) else {
        return Ok(None);
    };
    Ok(match type_name {
        "Date" => parsed.to_date().ok().map(Time::Date),
        "DateTime" => parsed.to_datetime().ok().map(Time::DateTime),
        "Instant" => parsed
            .to_timestamp()
            .ok()
            .or_else(|| parsed.to_zoned().ok().map(|zoned| zoned.timestamp()))
            .map(Time::Instant),
        _ => None,
    })
}

/// The evaluation clock and zone: fixed once per evaluation, so two reads of
/// `now()` in one evaluation agree.
#[derive(Debug, Clone, Default)]
pub struct Clock {
    /// `--now`: the instant every evaluation reads, instead of the host's.
    pub now: Option<Timestamp>,
    /// `--zone`: the zone every evaluation reads, instead of `TZ` or the
    /// host's.
    pub zone: Option<String>,
}

/// The clock the host configured: `--now` and `--zone`.
static CLOCK: Mutex<Clock> = Mutex::new(Clock {
    now: None,
    zone: None,
});

thread_local! {
    /// The instant, zone and zone name the evaluation on this thread read.
    /// An evaluation runs on one thread, so two evaluating at once each
    /// keep their own.
    static READ: std::cell::RefCell<Option<(Timestamp, TimeZone, String)>> =
        const { std::cell::RefCell::new(None) };
}

/// Fix the evaluation clock: `--now` and `--zone`.
pub fn set_clock(clock: Clock) {
    *CLOCK.lock().unwrap_or_else(PoisonError::into_inner) = clock;
    begin_evaluation();
}

/// Start an evaluation: its first read of the clock reads it afresh.
pub fn begin_evaluation() {
    READ.with(|read| *read.borrow_mut() = None);
}

/// The evaluation instant, zone and zone name, read once per evaluation.
///
/// # Errors
/// A `--zone` or `TZ` that names no zone.
pub fn evaluation_clock() -> Result<(Timestamp, TimeZone, String), String> {
    if let Some(read) = READ.with(|read| read.borrow().clone()) {
        return Ok(read);
    }
    let clock = CLOCK.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let now = clock.now.unwrap_or_else(Timestamp::now);
    let named = clock
        .zone
        .or_else(|| std::env::var("TZ").ok().filter(|name| !name.is_empty()));
    let (zone, name) = match named {
        Some(name) => (zone(&name)?, name),
        None => {
            let system = TimeZone::try_system().unwrap_or(TimeZone::UTC);
            let name = system.iana_name().unwrap_or("UTC").to_owned();
            (system, name)
        }
    };
    let read = (now, zone, name);
    READ.with(|held| *held.borrow_mut() = Some(read.clone()));
    Ok(read)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_type_reads_its_own_text_and_writes_it_back() {
        for (ty, text) in [
            ("Date", "2026-10-24"),
            ("DateTime", "2026-10-24T09:30:00"),
            ("DateTime", "2026-10-24T09:30:00.25"),
            ("Instant", "2026-10-24T07:30:00Z"),
            ("Duration", "PT1H30M"),
            ("Duration", "-PT0.5S"),
            ("Duration", "PT0S"),
        ] {
            let value = Time::parse_as(ty, text).unwrap_or_else(|| panic!("{text}"));
            assert_eq!(value.to_string(), text);
        }
    }

    #[test]
    fn the_constructors_refuse_other_forms() {
        for (ty, text) in [
            ("Date", "2026-10-24T09:30:00"),
            ("Date", "2026-02-30"),
            ("Date", "24.10.2026"),
            ("DateTime", "2026-10-24"),
            ("DateTime", "2026-10-24T09:30:00Z"),
            ("DateTime", "2026-10-24T24:00:00"),
            ("Instant", "2026-10-24T07:30:00"),
            ("Duration", "P1Y"),
            ("Duration", "P1M"),
            ("Duration", "PT"),
            ("Duration", "1h"),
        ] {
            assert_eq!(Time::parse_as(ty, text), None, "{text}");
        }
    }

    #[test]
    fn a_data_source_may_write_the_sql_form() {
        for (ty, text, read) in [
            ("DateTime", "2026-10-24 09:30:00", "2026-10-24T09:30:00"),
            ("Instant", "2026-10-24 09:30:00+02", "2026-10-24T07:30:00Z"),
            (
                "Instant",
                "2026-10-24 07:30:00.5+00:00",
                "2026-10-24T07:30:00.5Z",
            ),
            ("Date", " 2026-10-24 ", "2026-10-24"),
            ("Instant", "2026-10-24 07:30:00", "2026-10-24T07:30:00Z"),
        ] {
            assert_eq!(Time::read_as(ty, text).unwrap().to_string(), read, "{text}");
        }
        assert_eq!(Time::parse_as("DateTime", "2026-10-24 09:30:00"), None);
    }

    #[test]
    fn days_and_weeks_are_fixed_lengths() {
        let parsed = parse_duration("P1W2DT3H").unwrap();
        assert_eq!(duration_text(parsed), "PT219H");
        let offset = parse_instant("2026-10-24T09:30:00+02:00").unwrap();
        assert_eq!(offset.to_string(), "2026-10-24T07:30:00Z");
    }

    #[test]
    fn truncation_starts_the_unit() {
        let date = parse_date("2026-11-12").unwrap();
        assert_eq!(truncate_date(date, Unit::Week).to_string(), "2026-11-09");
        assert_eq!(truncate_date(date, Unit::Quarter).to_string(), "2026-10-01");
        assert_eq!(truncate_date(date, Unit::Hour), date);
    }

    #[test]
    fn a_gap_moves_forward_and_an_overlap_takes_the_earlier_instant() {
        let berlin = zone("Europe/Berlin").unwrap();
        let gap = parse_datetime("2026-03-29T02:30:00").unwrap();
        assert_eq!(
            in_zone(gap, &berlin).unwrap().to_string(),
            "2026-03-29T01:30:00Z"
        );
        let overlap = parse_datetime("2026-10-25T02:30:00").unwrap();
        assert_eq!(
            in_zone(overlap, &berlin).unwrap().to_string(),
            "2026-10-25T00:30:00Z"
        );
    }
}
