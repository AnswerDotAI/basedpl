use crate::cancel::check;
use crate::{Described, Error, Number, Span, Value, array::generated_len, data::{Columns, text as string}, keyed, number::Numeric};
use rustymath::{buffer::DType, error::Kind};
use std::sync::Arc;

enum Fill { Number(Number), Text(String) }

struct Options {
    common: crate::data::Options,
    fill: Option<Number>,
    ifill: Option<Number>,
    separator: u8,
    quote: Option<u8>,
    escape: Option<u8>,
    double_quote: bool,
    trim: bool,
    decimal: char,
    thousands: Option<char>,
    /// The width of the floats that fields read as, the session's default width.
    width: rustymath::buffer::FloatWidth,
}

impl Options {
    fn new(left: Option<&Value>, import: bool) -> Result<Self, Error> {
        let mut allowed = vec!["separator", "quote", "escapechar", "doublequote", "decimal", "thousands", "trim", "header", "fill", "ifill", "fills"];
        allowed.extend(if import { &["missing", "types"][..] } else { &["forcequotes", "eol"][..] });
        let common = crate::data::Options::new(if import { "•csv" } else { "•csv⁻" }, left, None, &allowed)?;
        let fill = common.values.contains_key("fill").then(|| common.fill()).transpose()?;
        let ifill = common.values.get("ifill")
            .map(|v| v.as_number().filter(|n| n.integer().is_ok()).ok_or_else(|| Error::domain("CSV ifill must be a whole real number"))).transpose()?;
        let width = crate::number::Numeric::current().width;
        let mut opts = Self { common, fill, ifill, separator: b',', quote: Some(b'"'), escape: None, double_quote: true, trim: false, decimal: '.',
            thousands: None, width };
        let byte = |c: Option<char>| -> Result<Option<u8>, Error> {
            c.map(|c| {
                if c.is_ascii() && !matches!(c, '\r' | '\n' | '\0') { Ok(c as u8) }
                else { Err(Error::domain("CSV separator, quote and escape must be non-newline ASCII characters")) }
            }).transpose()
        };
        opts.separator = byte(opts.character("separator", Some(','))?)?.ok_or_else(|| Error::domain("CSV needs a separator"))?;
        opts.quote = byte(opts.character("quote", Some('"'))?)?;
        opts.escape = byte(opts.character("escapechar", None)?)?;
        opts.double_quote = opts.common.boolean("doublequote", true)?;
        opts.trim = opts.common.boolean("trim", false)?;
        opts.decimal = opts.character("decimal", Some('.'))?.filter(|c| matches!(c, '.' | ',')).ok_or_else(|| Error::domain("CSV decimal must be '.' or ','"))?;
        opts.thousands = opts.character("thousands", None)?;
        let special: Vec<_> = [Some(opts.separator), opts.quote, opts.escape].into_iter().flatten().collect();
        if special.iter().enumerate().any(|(i, c)| special[..i].contains(c)) { return Err(Error::domain("CSV separator, quote and escape must differ")); }
        if opts.thousands.is_some_and(|c| c == opts.decimal || c.is_ascii_digit() || matches!(c, '+' | '-' | 'e' | 'E' | '\r' | '\n')) {
            return Err(Error::domain("invalid CSV thousands separator"));
        }
        Ok(opts)
    }

    fn columns(&self, key: &str, names: &[Option<&str>]) -> Result<Vec<Option<Value>>, Error> {
        match self.common.columns(key)? {
            Columns::Named(named) => {
                if let Some(k) = named.keys().find(|k| !names.contains(&Some(k.as_str()))) {
                    return Err(Error::domain(format!("CSV {key} names no column {k:?}")));
                }
                Ok(names.iter().map(|n| n.and_then(|n| named.get(n).cloned())).collect())
            }
            Columns::Ordered(values) if values.len() == names.len() => Ok(values.into_iter().map(Some).collect()),
            Columns::Ordered(values) => Err(Error::new(Kind::Length, format!("CSV {key} gives {} values for {} columns", values.len(), names.len()))),
        }
    }

    fn fill(&self, specified: Option<Value>, integer: bool, numeric: bool) -> Result<Option<Fill>, Error> {
        let fill = specified.or_else(|| self.ifill.as_ref().filter(|_| integer).or_else(|| self.fill.as_ref().filter(|_| numeric)).cloned().map(Value::Number));
        fill.map(|v| match v {
            Value::Number(n) if n.as_complex().is_none() => Ok(Fill::Number(n)),
            _ => string(&v).map(Fill::Text).map_err(|_| Error::domain("CSV fills must be real numbers or text")),
        }).transpose()
    }

    fn character(&self, key: &str, default: Option<char>) -> Result<Option<char>, Error> {
        let Some(v) = self.common.values.get(key) else { return Ok(default); };
        let s = string(v)?;
        let mut chars = s.chars();
        let c = chars.next();
        if chars.next().is_some() { return Err(Error::new(Kind::Length, format!("CSV {key} needs one character or ''"))); }
        Ok(c)
    }

    fn number(&self, field: &str) -> Option<Number> {
        let mut field = field.to_owned();
        if let Some(sep) = self.thousands {
            if field.contains(sep) {
                let integer_end = field.find([self.decimal, 'e', 'E']).unwrap_or(field.len());
                let integer = field[..integer_end].trim_start_matches(['+', '-']);
                let groups: Vec<_> = integer.split(sep).collect();
                if groups[0].is_empty() || groups[0].len() > 3 || groups.iter().any(|s| !s.bytes().all(|b| b.is_ascii_digit()))
                    || groups[1..].iter().any(|s| s.len() != 3) || field[integer_end..].contains(sep) { return None; }
                field = field.replace(sep, "");
            }
        }
        if self.decimal != '.' {
            if field.contains('.') { return None; }
            field = field.replace(self.decimal, ".");
        }
        let digits = field.trim_start_matches(['+', '-']);
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) { return Number::decimal(&field).map(|n| Number::data_integer(n, self.width)); }
        let n = Number::float(field.parse::<f64>().ok()?, self.width);
        if n.is_infinite() && !matches!(digits.to_ascii_lowercase().as_str(), "inf" | "infinity") { return None; }
        Some(n)
    }
}

/// `•csv`: parse CSV text into a keyed vector of columns.
pub(crate) fn parse(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> { import(&string(right)?, &Options::new(left, true)?) }

/// `•csv⁻`: CSV text for a vector of columns.
pub(crate) fn serialize(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> { export(right, &Options::new(left, false)?) }

fn import(source: &str, opts: &Options) -> Result<Value, Error> {
    let header = opts.common.boolean("header", true)?;
    let default_fill = Numeric::current().float(f64::NAN);
    let mut missing = vec![String::new()];
    if let Some(v) = opts.common.values.get("missing") {
        if let Some(s) = keyed::name(v) { missing.push(s.to_string()); } else { for s in v.elements() { missing.push(string(&s)?); } }
    }
    let mut builder = csv::ReaderBuilder::new();
    builder.has_headers(false).delimiter(opts.separator).quoting(opts.quote.is_some()).quote(opts.quote.unwrap_or(b'"')).escape(opts.escape)
        .double_quote(opts.double_quote).trim(if opts.trim { csv::Trim::All } else { csv::Trim::None });
    let mut reader = builder.from_reader(source.as_bytes());
    let mut headers = None;
    let mut columns: Vec<Vec<String>> = Vec::new();
    for (row, record) in reader.records().enumerate() {
        check()?;
        let record = record.map_err(|e| Error::new(Kind::Length, format!("CSV {e}")))?;
        if row == 0 {
            columns = vec![Vec::new(); record.len()];
            if header { headers = Some(record.iter().map(Arc::<str>::from).collect::<Vec<_>>()); continue; }
        }
        generated_len(&[columns[0].len() + 1, columns.len()])?;
        for (column, s) in columns.iter_mut().zip(record.iter()) { column.push(s.to_owned()); }
    }
    let names: Vec<_> = (0..columns.len()).map(|j| headers.as_ref().map(|h| &*h[j])).collect();
    let types = opts.columns("types", &names)?;
    let fills = opts.columns("fills", &names)?;
    let mut result = Vec::with_capacity(columns.len());
    for (j, ((cells, typed), specified)) in columns.into_iter().zip(types).zip(fills).enumerate() {
        check()?;
        let name = headers.as_ref().map_or_else(|| (j + 1).to_string(), |h| h[j].to_string());
        let text = typed.as_ref().is_some_and(|t| keyed::name(t).is_some());
        let absent: Vec<bool> = cells.iter().map(|s| missing.contains(s)).collect();
        let parsed: Vec<_> = cells.iter().zip(&absent).map(|(s, &a)| if text || a { None } else { opts.number(s) }).collect();
        let is_numeric = !text && (typed.is_some() || (parsed.iter().any(Option::is_some) && parsed.iter().zip(&absent).all(|(n, &a)| n.is_some() || a)));
        let integer = opts.ifill.is_some() && specified.is_none() && typed.as_ref().map_or_else(
                || is_numeric && parsed.iter().flatten().all(|n| matches!(n.dtype(), Some(DType::Integer(_)))), |v| matches!(v.dtype(), Some(DType::Integer(_)))
            );
        let fill = opts.fill(specified, integer, is_numeric)?;
        let column = if is_numeric {
            let fill = match fill {
                Some(Fill::Number(n)) => n, None => default_fill.clone(), _ => return Err(Error::domain(format!("CSV column {name:?} needs a numeric fill")))
            };
            let mut numbers = Vec::with_capacity(cells.len());
            for (i, (s, n)) in cells.iter().zip(parsed).enumerate() {
                numbers.push(Value::Number(match n {
                    Some(n) => n, None if absent[i] => fill.clone(),
                    None => return Err(Error::domain(format!("CSV row {}, column {}: invalid number {s:?}", i + 1 + usize::from(header), j + 1)))
                }));
            }
            crate::data::typed_column(
                "•csv", &name, crate::data::imported(vec![cells.len()], numbers, |i| absent[i], Value::integer(0)).described("invalid CSV column")?,
                typed.as_ref(), absent.contains(&true))?
        } else {
            let fill = match fill {
                Some(Fill::Text(s)) => s, None => String::new(), _ => return Err(Error::domain(format!("CSV column {name:?} needs a text fill")))
            };
            let mut texts = keyed::Texts::default();
            keyed::texts(&[cells.len()], cells.iter().zip(&absent).map(|(s, &a)| texts.text(if a { &fill } else { s })).collect())
                .described("invalid CSV column")?
        };
        result.push(column);
    }
    match headers {
        Some(names) => keyed::vector(names, result), None => Value::from_parts(vec![result.len()], result, keyed::text(""))
    }.described("CSV headers must be unique")
}

fn export(table: &Value, opts: &Options) -> Result<Value, Error> {
    if table.shape().len() != 1 { return Err(Error::new(Kind::Rank, "CSV export expects a vector of column vectors")); }
    let columns: Vec<_> = table.elements().collect();
    if columns.iter().any(|c| c.shape().len() != 1) { return Err(Error::new(Kind::Rank, "CSV columns must be vectors")); }
    let rows = columns.first().map_or(0, Value::len);
    if columns.iter().any(|c| c.len() != rows) { return Err(Error::new(Kind::Length, "CSV columns must have equal lengths")); }
    let header = opts.common.boolean("header", table.keys(0).is_some())?;
    let names: Vec<_> = (0..columns.len()).map(|j| table.keys(0).and_then(|k| k.names()[j].as_deref())).collect();
    let fills = opts.columns("fills", &names)?.into_iter().zip(&columns)
        .map(|(given, c)| opts.fill(given, opts.ifill.is_some() && matches!(c.dtype(), Some(DType::Integer(_))), true)).collect::<Result<Vec<_>, _>>()?;
    let force = match opts.common.values.get("forcequotes") {
        None => false,
        Some(v) => match v.as_number().and_then(|n| n.integer().ok()) {
            Some(0) => false, Some(2) => true, _ => return Err(Error::domain("CSV forcequotes is 0 (as needed) or 2 (all fields)"))
        },
    };
    if force && opts.quote.is_none() { return Err(Error::domain("CSV forcequotes needs quote")); }
    let ending = opts.common.text("eol", Some("\n"))?;
    let terminator = match ending.as_str() {
        "\n" => csv::Terminator::Any(b'\n'), "\r\n" => csv::Terminator::CRLF, _ => return Err(Error::domain("CSV eol must be LF or CRLF"))
    };
    let mut writer = csv::WriterBuilder::new().delimiter(opts.separator).quote(opts.quote.unwrap_or(b'"')).quote_style(if opts.quote.is_none() {
        csv::QuoteStyle::Never
    } else if force || opts.escape.is_some() { csv::QuoteStyle::Always }
    else { csv::QuoteStyle::Necessary }).double_quote(opts.double_quote).escape(opts.escape.unwrap_or(b'\\')).terminator(terminator).from_writer(Vec::new());
    let write = |writer: &mut csv::Writer<Vec<u8>>, fields: Vec<String>| -> Result<(), Error> {
        for field in &fields {
            if opts.quote.is_none() && (field.contains([opts.separator as char, '\r', '\n']) || fields.len() == 1 && field.is_empty()) {
                return Err(Error::domain("CSV field requires quoting"));
            }
            if !opts.double_quote && opts.escape.is_none() && opts.quote.is_some_and(|q| field.contains(q as char)) {
                return Err(Error::domain("CSV quote requires doublequote or escapechar"));
            }
        }
        let fields = fields.into_iter().map(|s| match (opts.quote, opts.escape) {
            (Some(_), Some(e)) => s.replace(e as char, &(e as char).to_string().repeat(2)), _ => s
        });
        writer.write_record(fields).map_err(|e| Error::domain(format!("CSV {e}")))
    };
    if header {
        let keys = table.keys(0).filter(|k| k.complete()).ok_or_else(|| Error::domain("CSV headers require a key for every column"))?;
        write(&mut writer, keys.names().iter().flatten().map(|s| s.to_string()).collect())?;
    }
    for i in 0..rows {
        check()?;
        let fields = columns.iter().enumerate().map(|(j, c)| {
            field(&c.at(i), opts, fills[j].as_ref()).map_err(|mut e| {
                e.message = format!("CSV row {}, column {}: {}", i + 1, j + 1, e.message);
                e
            })
        }).collect::<Result<_, _>>()?;
        write(&mut writer, fields)?;
    }
    let bytes = writer.into_inner().map_err(|e| Error::domain(format!("CSV {e}")))?;
    Ok(keyed::text(&String::from_utf8(bytes).unwrap()))
}

fn field(v: &Value, opts: &Options, fill: Option<&Fill>) -> Result<String, Error> {
    let mut s = if let Some(s) = keyed::name(v) {
        if matches!(fill, Some(Fill::Text(f)) if &*s == f) { return Ok(String::new()); }
        if opts.trim { s.trim().to_owned() } else { s.to_string() }
    } else if let Value::Number(n) = v {
        if n.is_nan() || matches!(fill, Some(Fill::Number(f)) if n.grade_order(f).is_eq()) { return Ok(String::new()); }
        if let Some(t) = n.integer_text() { t }
        else if let Some(n) = n.as_float() { let s = n.to_string(); if n.is_finite() && !s.contains(['.', 'e', 'E']) { format!("{s}.0") }
        else { s } }
        else { return Err(Error::domain("CSV cells must be real floats, integers or strings")); }
    } else { return Err(Error::domain("CSV cells must be real floats, integers or strings")); };
    if matches!(v, Value::Number(_)) {
        if opts.decimal != '.' { s = s.replace('.', &opts.decimal.to_string()); }
        if let Some(sep) = opts.thousands {
            let end = s.find([opts.decimal, 'e', 'E']).unwrap_or(s.len());
            let start = usize::from(s.starts_with('-'));
            if s[start..end].bytes().all(|b| b.is_ascii_digit()) { for pos in (start + 1..end).rev().filter(|&p| (end - p) % 3 == 0) { s.insert(pos, sep); } }
        }
    }
    Ok(s)
}
