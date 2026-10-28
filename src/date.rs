//! `•date` reads a moment, in seconds since the Unix epoch, from text or from a record of fields. `•date⁻¹` writes a moment as a record, or as text with a pattern. Patterns, field names and ISO 8601 text follow chrono.
use crate::{Described, Error, Number, Span, Value, data::{Options, text}, keyed, primitive::real};
use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone, Timelike, Utc};
use rustymath::buffer::FloatWidth;
use std::fmt::Write;

/// Seconds since the Unix epoch, from the system clock.
pub(crate) fn now() -> f64 { seconds(Utc::now()) }

/// The moment `t` in seconds since the Unix epoch.
pub(crate) fn seconds(t: DateTime<Utc>) -> f64 { t.timestamp() as f64 + f64::from(t.timestamp_subsec_nanos()) / 1e9 }

/// The moment `x` seconds after the Unix epoch, or `None` for a number outside chrono's range.
fn moment(x: f64) -> Option<DateTime<Utc>> {
    if !x.is_finite() { return None; }
    let whole = x.floor();
    // Rounding the fraction can reach a whole second.
    DateTime::from_timestamp(whole as i64, ((x - whole) * 1e9).round().min(999_999_999.0) as u32)
}

/// The fields of a record that `•date⁻¹` writes, named after chrono's accessors. `•date` reads the first seven. The last three follow from them, and `•date` ignores them.
const FIELDS: [&str; 10] = ["year", "month", "day", "hour", "minute", "second", "nanosecond", "weekday", "ordinal", "iso_week"];

/// The time zone that text and fields are in: a fixed offset east of UTC, or the system's local zone, which can change with the date.
enum Zone { Fixed(FixedOffset), Local }

impl Zone {
    /// The `zone` option: seconds east of UTC, or `"local"`. UTC without one.
    fn new(opts: &Options) -> Result<Self, Error> {
        let Some(zone) = opts.values.get("zone") else { return Ok(Self::Fixed(Utc.fix())) };
        if keyed::name(zone).as_deref() == Some("local") { return Ok(Self::Local); }
        let offset = real(zone)?;
        FixedOffset::east_opt(offset as i32).filter(|_| offset.fract() == 0.0).map(Self::Fixed)
            .ok_or_else(|| Error::domain("•date zone must be \"local\" or whole seconds east of UTC, less than a day"))
    }
    /// The date and time at moment `t` in this zone.
    fn at(&self, t: DateTime<Utc>) -> DateTime<FixedOffset> {
        match self { Self::Fixed(offset) => t.with_timezone(offset), Self::Local => t.with_timezone(&Local).fixed_offset() }
    }
    /// The moment that the date and time `naive` names in this zone. A local time that a clock change repeats names its earlier moment.
    fn moment(&self, naive: NaiveDateTime) -> Option<DateTime<Utc>> {
        match self {
            Self::Fixed(offset) => offset.from_local_datetime(&naive).single().map(|t| t.to_utc()),
            Self::Local => Local.from_local_datetime(&naive).earliest().map(|t| t.to_utc()),
        }
    }
}

/// The ISO 8601 forms that `•date` reads without a pattern: RFC 3339, then a date and time with no offset, then a date alone.
const ISO: [&str; 3] = ["%+", "%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d"];

/// The moment that `text` names in the layout `pattern`. Text with no offset is in `zone`, and text with no time is at midnight.
fn parse(text: &str, pattern: &str, zone: &Zone) -> Option<DateTime<Utc>> {
    if let Ok(t) = DateTime::parse_from_str(text, pattern) { return Some(t.to_utc()); }
    let naive = NaiveDateTime::parse_from_str(text, pattern).or_else(|_| NaiveDate::parse_from_str(text, pattern).map(|d| d.and_time(NaiveTime::MIN)));
    zone.moment(naive.ok()?)
}

/// The date and time that the fields year, month, day, hour, minute, second and nanosecond give, or `None` when they don't name one.
fn date_time(fields: &[i64]) -> Option<NaiveDateTime> {
    let [year, month, day, hour, minute, second, nano] = fields else { return None };
    let small = |n: &i64| u32::try_from(*n).ok();
    NaiveDate::from_ymd_opt(i32::try_from(*year).ok()?, small(month)?, small(day)?)?.and_hms_nano_opt(
        small(hour)?, small(minute)?, small(second)?, small(nano)?)
}

/// `•date Y`: the moment that the text `Y` names, or one for each text in `Y`, or one for each position of a record of fields. A pattern, or the `pattern` and `zone` options, go on the left.
pub(crate) fn read(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let opts = Options::new("•date", left, Some("pattern"), &["pattern", "zone"])?;
    let zone = Zone::new(&opts)?;
    let pattern = opts.values.get("pattern").map(|p| text(p)).transpose()?;
    let read = |text: &str| match &pattern { Some(p) => parse(text, p, &zone), None => ISO.iter().find_map(|p| parse(text, p, &zone)) };
    let (shape, moments) = if right.keys(0).is_some() {
        let (shape, fields) = keyed::fields(right).described("a date record's fields must agree")?;
        let columns = keyed::slots(fields, &FIELDS, 7).map_err(|name| Error::domain(format!("unknown date field: {name}")))?;
        if columns[0].is_none() { return Err(Error::domain("a date record needs a year")); }
        let field = |i: usize, p: usize| -> Result<i64, Error> {
            // A missing month or day is 1, and a missing time field is 0.
            let Some(items) = &columns[i] else { return Ok([0, 1, 1, 0, 0, 0, 0][i]) };
            match &items[p] {
                Value::Number(n) => n.integer().ok(), _ => None
            }.ok_or_else(|| Error::domain(format!("date field {} must be a whole number", FIELDS[i])))
        };
        let moments = (0..crate::array::count(&shape)).map(|p| {
            let values = (0..7).map(|i| field(i, p)).collect::<Result<Vec<_>, _>>()?;
            Ok(date_time(&values).and_then(|n| zone.moment(n)))
        });
        (shape, moments.collect::<Result<Vec<_>, Error>>()?)
    } else {
        let (shape, texts) = keyed::text_items(right).ok_or_else(|| Error::domain("•date reads text or a record of fields"))?;
        (shape, texts.iter().map(|t| read(t)).collect())
    };
    let seconds = moments.into_iter().map(|m| m.map(seconds).ok_or_else(|| Error::domain("invalid date"))).collect::<Result<Vec<_>, _>>()?;
    // Seconds since 1970 stay 64-bit, because a 32-bit float holds them only to within about two minutes.
    let second = |s| Value::Number(Number::float(s, FloatWidth::F64));
    Value::shaped(&shape, seconds.into_iter().map(second).collect(), second(0.0)).described("invalid date")
}

/// `•date⁻¹`'s options. The browser build has no `locale`, which keeps chrono's locale data out of it.
const WRITE_OPTIONS: &[&str] = if cfg!(web) { &["pattern", "zone"] } else { &["pattern", "zone", "locale"] };

/// `•date⁻¹ T`: a record of the fields of each moment in `T`. With a pattern, the text of each moment. The `zone` option gives the time zone, and the `locale` option the language of names in the text.
pub(crate) fn write(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let opts = Options::new("•date⁻¹", left, Some("pattern"), WRITE_OPTIONS)?;
    let zone = Zone::new(&opts)?;
    #[cfg(not(web))]
    let locale = match opts.values.get("locale") {
        Some(l) => Some(chrono::Locale::try_from(text(l)?.as_str()).map_err(|_| Error::domain("unknown •date⁻¹ locale"))?), None => None
    };
    let moments = right.elements().map(|e| real(&e).and_then(|x| moment(x).map(|t| zone.at(t)).ok_or_else(|| Error::domain("invalid date"))));
    let moments = moments.collect::<Result<Vec<_>, _>>()?;
    let shape = right.shape();
    let Some(pattern) = opts.values.get("pattern").map(|p| text(p)).transpose()? else {
        #[cfg(not(web))]
        if locale.is_some() { return Err(Error::domain("a •date⁻¹ locale needs a pattern")); }
        let rows = moments.iter().map(|t| {
            let fields = [t.year(), t.month() as i32, t.day() as i32, t.hour() as i32, t.minute() as i32, t.second() as i32, t.nanosecond() as i32];
            let derived = [t.weekday().number_from_monday(), t.ordinal(), t.iso_week().week()].map(|n| n as i32);
            fields.into_iter().chain(derived).map(|n| Value::integer(n.into())).collect()
        });
        return keyed::table(&FIELDS, shape, rows.collect(), vec![Value::integer(0); FIELDS.len()]).described("invalid date record");
    };
    let texts = moments.iter().map(|t| {
        let mut s = String::new();
        #[cfg(not(web))]
        let written = match locale { Some(locale) => write!(s, "{}", t.format_localized(&pattern, locale)), None => write!(s, "{}", t.format(&pattern)) };
        #[cfg(web)]
        let written = write!(s, "{}", t.format(&pattern));
        written.map(|()| keyed::text(&s)).map_err(|_| Error::domain("invalid •date⁻¹ pattern"))
    });
    keyed::texts(shape, texts.collect::<Result<_, _>>()?).described("invalid date text")
}
