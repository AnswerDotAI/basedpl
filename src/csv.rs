use crate::{array::generated_len, data::text as string, execution::Context, keyed, Error, ErrorKind, Number, Value};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::ToPrimitive;
use std::sync::Arc;

struct Options {
    common: crate::data::Options,
    separator: u8,
    quote: Option<u8>,
    escape: Option<u8>,
    double_quote: bool,
    trim: bool,
    decimal: char,
    thousands: Option<char>,
}

impl Options {
    fn new(right: &Value, import: bool, span: &Context<'_>) -> Result<Self, Error> {
        let mut allowed = vec!["separator", "quotechar", "escapechar", "doublequote", "decimal", "thousands", "trim", "header", "fill"];
        allowed.extend(if import { &["source", "text_columns", "numeric_columns", "missing"][..] } else { &["forcequotes", "lineending"][..] });
        let common = crate::data::Options::new("•CSV", right, import.then_some("source"), &allowed, span)?;
        let mut opts = Self { common, separator: b',', quote: Some(b'"'), escape: None, double_quote: true, trim: false, decimal: '.', thousands: None };
        let byte = |c: Option<char>| -> Result<Option<u8>, Error> {
            c.map(|c| {
                if c.is_ascii() && !matches!(c, '\r' | '\n' | '\0') { Ok(c as u8) } else { Err(span.error(ErrorKind::Domain, "CSV separator, quote and escape must be non-newline ASCII characters")) }
            })
            .transpose()
        };
        opts.separator = byte(opts.character("separator", Some(','), span)?)?.ok_or_else(|| span.error(ErrorKind::Domain, "CSV needs a separator"))?;
        opts.quote = byte(opts.character("quotechar", Some('"'), span)?)?;
        opts.escape = byte(opts.character("escapechar", None, span)?)?;
        opts.double_quote = opts.common.boolean("doublequote", true, span)?;
        opts.trim = opts.common.boolean("trim", false, span)?;
        opts.decimal = opts
            .character("decimal", Some('.'), span)?
            .filter(|c| matches!(c, '.' | ','))
            .ok_or_else(|| span.error(ErrorKind::Domain, "CSV decimal must be '.' or ','"))?;
        opts.thousands = opts.character("thousands", None, span)?;
        let special: Vec<_> = [Some(opts.separator), opts.quote, opts.escape].into_iter().flatten().collect();
        if special.iter().enumerate().any(|(i, c)| special[..i].contains(c)) {
            return Err(span.error(ErrorKind::Domain, "CSV separator, quote and escape must differ"));
        }
        if opts.thousands.is_some_and(|c| c == opts.decimal || c.is_ascii_digit() || matches!(c, '+' | '-' | 'e' | 'E' | '\r' | '\n')) {
            return Err(span.error(ErrorKind::Domain, "invalid CSV thousands separator"));
        }
        Ok(opts)
    }

    fn character(&self, key: &str, default: Option<char>, span: &Context<'_>) -> Result<Option<char>, Error> {
        let Some(v) = self.common.values.get(key) else { return Ok(default); };
        let s = string(v, span)?;
        let mut chars = s.chars();
        let c = chars.next();
        if chars.next().is_some() { return Err(span.error(ErrorKind::Length, format!("CSV {key} needs one character or ''"))); }
        Ok(c)
    }

    fn columns(&self, key: &str, width: usize, headers: Option<&[Arc<str>]>, span: &Context<'_>) -> Result<Vec<bool>, Error> {
        let mut selected = vec![false; width];
        if let Some(v) = self.common.values.get(key) {
            if v.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "CSV column selectors must be a vector")); }
            let selectors = if keyed::name(v).is_some() { vec![v.clone()] } else { v.elements().collect() };
            for v in selectors {
                let i = if let Some(name) = keyed::name(&v) { headers.and_then(|h| h.iter().position(|s| s == &name)) } else { v.as_number().and_then(|n| n.nonnegative_integer().ok()).and_then(|n| n.checked_sub(1)) };
                let i = i.filter(|&i| i < width).ok_or_else(|| span.error(ErrorKind::Index, "unknown CSV column"))?;
                selected[i] = true;
            }
        }
        Ok(selected)
    }

    fn number(&self, field: &str) -> Option<Number> {
        let mut field = field.to_owned();
        if let Some(sep) = self.thousands {
            if field.contains(sep) {
                let integer_end = field.find([self.decimal, 'e', 'E']).unwrap_or(field.len());
                let integer = field[..integer_end].trim_start_matches(['+', '-']);
                let groups: Vec<_> = integer.split(sep).collect();
                if groups[0].is_empty()
                    || groups[0].len() > 3
                    || groups.iter().any(|s| !s.bytes().all(|b| b.is_ascii_digit()))
                    || groups[1..].iter().any(|s| s.len() != 3)
                    || field[integer_end..].contains(sep)
                { return None; }
                field = field.replace(sep, "");
            }
        }
        if self.decimal != '.' {
            if field.contains('.') { return None; }
            field = field.replace(self.decimal, ".");
        }
        let digits = field.trim_start_matches(['+', '-']);
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            if let Ok(n) = field.parse::<i64>() { return Some(Number::from_integer(n)); }
            return Number::try_from(BigRational::from_integer(field.parse::<BigInt>().ok()?)).ok();
        }
        let n = field.parse::<f64>().ok()?;
        if n.is_infinite() && !matches!(digits.to_ascii_lowercase().as_str(), "inf" | "infinity") { return None; }
        Number::try_from(n).ok()
    }
}

pub(crate) fn call(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = Options::new(right, left.is_none(), span)?;
    match left { None => import(&opts, span), Some(table) => export(table, &opts, span) }
}

fn import(opts: &Options, span: &Context<'_>) -> Result<Value, Error> {
    let source = opts.common.text("source", span)?;
    let header = opts.common.boolean("header", true, span)?;
    let fill = opts.common.fill(span)?;
    let mut missing = vec![String::new()];
    if let Some(v) = opts.common.values.get("missing") {
        if let Some(s) = keyed::name(v) { missing.push(s.to_string()); } else { for s in v.elements() { missing.push(string(&s, span)?); } }
    }
    let mut builder = csv::ReaderBuilder::new();
    builder
        .has_headers(false)
        .delimiter(opts.separator)
        .quoting(opts.quote.is_some())
        .quote(opts.quote.unwrap_or(b'"'))
        .escape(opts.escape)
        .double_quote(opts.double_quote)
        .trim(if opts.trim { csv::Trim::All } else { csv::Trim::None });
    let mut reader = builder.from_reader(source.as_bytes());
    let mut headers = None;
    let mut columns: Vec<Vec<String>> = Vec::new();
    for (row, record) in reader.records().enumerate() {
        span.check()?;
        let record = record.map_err(|e| span.error(ErrorKind::Length, format!("CSV {e}")))?;
        if row == 0 {
            columns = vec![Vec::new(); record.len()];
            if header {
                headers = Some(record.iter().map(Arc::<str>::from).collect::<Vec<_>>());
                continue;
            }
        }
        generated_len(&[columns[0].len() + 1, columns.len()]).map_err(|k| span.error(k, "CSV table is too large"))?;
        for (column, s) in columns.iter_mut().zip(record.iter()) { column.push(s.to_owned()); }
    }
    let text_columns = opts.columns("text_columns", columns.len(), headers.as_deref(), span)?;
    let numeric_columns = opts.columns("numeric_columns", columns.len(), headers.as_deref(), span)?;
    let mut result = Vec::with_capacity(columns.len());
    for (j, cells) in columns.into_iter().enumerate() {
        span.check()?;
        if text_columns[j] && numeric_columns[j] { return Err(span.error(ErrorKind::Domain, "CSV column selected as both text and numeric")); }
        let parsed: Vec<_> = cells.iter().map(|s| if text_columns[j] || missing.contains(s) { None } else { opts.number(s) }).collect();
        let is_numeric = numeric_columns[j]
            || (!text_columns[j] && parsed.iter().any(Option::is_some) && cells.iter().zip(&parsed).all(|(s, n)| n.is_some() || missing.contains(s)));
        let (data, prototype) = if is_numeric {
            let approximate = parsed.iter().flatten().any(|n| n.as_float().is_some());
            let mut numbers = Vec::with_capacity(cells.len());
            for (i, (s, n)) in cells.iter().zip(parsed).enumerate() {
                numbers.push(match n {
                    Some(n) => n,
                    None if missing.contains(s) => fill.clone(),
                    None => {
                        return Err(span.error(ErrorKind::Domain, format!("CSV row {}, column {}: invalid number {s:?}", i + 1 + usize::from(header), j + 1)))
                    }
                });
            }
            // Missing fill does not promote an otherwise exact column.
            let floats: Option<Vec<_>> = approximate.then(|| numbers.iter().map(lossless_float).collect()).flatten();
            let data = match floats {
                Some(values) => values.into_iter().map(|n| Value::Number(Number::try_from(n).unwrap())).collect(),
                None => numbers.into_iter().map(Value::Number).collect(),
            };
            (data, Value::Number(Number::from_integer(0)))
        } else { (cells.iter().map(|s| keyed::text(if missing.contains(s) { "" } else { s })).collect(), keyed::text("")) };
        result.push(Value::from_parts(vec![cells.len()], data, prototype).map_err(|k| span.error(k, "invalid CSV column"))?);
    }
    match headers { Some(names) => keyed::vector(names, result), None => Value::from_parts(vec![result.len()], result, keyed::text("")) }
    .map_err(|k| span.error(k, "CSV headers must be unique"))
}

fn lossless_float(n: &Number) -> Option<f64> {
    if let Some(f) = n.as_float() { return Some(f); }
    if let Some(i) = n.as_integer() {
        let f = i as f64;
        return (f as i128 == i as i128).then_some(f);
    }
    let rational = n.as_exact()?;
    let f = rational.to_f64()?;
    (BigRational::from_float(f)? == rational).then_some(f)
}

fn export(table: &Value, opts: &Options, span: &Context<'_>) -> Result<Value, Error> {
    if table.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, "CSV export expects a vector of column vectors")); }
    let columns: Vec<_> = table.elements().collect();
    if columns.iter().any(|c| c.shape().len() != 1) { return Err(span.error(ErrorKind::Rank, "CSV columns must be vectors")); }
    let rows = columns.first().map_or(0, Value::len);
    if columns.iter().any(|c| c.len() != rows) { return Err(span.error(ErrorKind::Length, "CSV columns must have equal lengths")); }
    let header = opts.common.boolean("header", table.keys(0).is_some(), span)?;
    let fill = opts.common.fill(span)?;
    let force = match opts.common.values.get("forcequotes") {
        None => false,
        Some(v) => match v.as_number().and_then(|n| n.integer().ok()) {
            Some(0) => false,
            Some(2) => true,
            _ => return Err(span.error(ErrorKind::Domain, "CSV forcequotes is 0 (as needed) or 2 (all fields)")),
        },
    };
    if force && opts.quote.is_none() { return Err(span.error(ErrorKind::Domain, "CSV forcequotes needs quotechar")); }
    let ending = opts.common.values.get("lineending").map(|v| string(v, span)).transpose()?.unwrap_or_else(|| "\n".into());
    let terminator = match ending.as_str() {
        "\n" => csv::Terminator::Any(b'\n'),
        "\r\n" => csv::Terminator::CRLF,
        _ => return Err(span.error(ErrorKind::Domain, "CSV lineending must be LF or CRLF")),
    };
    let mut writer = csv::WriterBuilder::new()
        .delimiter(opts.separator)
        .quote(opts.quote.unwrap_or(b'"'))
        .quote_style(if opts.quote.is_none() { csv::QuoteStyle::Never } else if force || opts.escape.is_some() { csv::QuoteStyle::Always } else { csv::QuoteStyle::Necessary })
        .double_quote(opts.double_quote)
        .escape(opts.escape.unwrap_or(b'\\'))
        .terminator(terminator)
        .from_writer(Vec::new());
    let write = |writer: &mut csv::Writer<Vec<u8>>, fields: Vec<String>| -> Result<(), Error> {
        for field in &fields {
            if opts.quote.is_none() && (field.contains([opts.separator as char, '\r', '\n']) || fields.len() == 1 && field.is_empty()) {
                return Err(span.error(ErrorKind::Domain, "CSV field requires quoting"));
            }
            if !opts.double_quote && opts.escape.is_none() && opts.quote.is_some_and(|q| field.contains(q as char)) {
                return Err(span.error(ErrorKind::Domain, "CSV quote requires doublequote or escapechar"));
            }
        }
        let fields = fields.into_iter().map(|s| match (opts.quote, opts.escape) {
            (Some(_), Some(e)) => s.replace(e as char, &(e as char).to_string().repeat(2)),
            _ => s,
        });
        writer.write_record(fields).map_err(|e| span.error(ErrorKind::Domain, format!("CSV {e}")))
    };
    if header {
        let keys = table.keys(0).ok_or_else(|| span.error(ErrorKind::Domain, "CSV headers require keyed columns"))?;
        write(&mut writer, keys.names().iter().map(|s| s.to_string()).collect())?;
    }
    for i in 0..rows {
        span.check()?;
        let fields = columns
            .iter()
            .enumerate()
            .map(|(j, c)| field(&c.at(i), opts, &fill, span).map_err(|e| span.error(e.kind, format!("CSV row {}, column {}: {}", i + 1, j + 1, e.message))))
            .collect::<Result<_, _>>()?;
        write(&mut writer, fields)?;
    }
    let bytes = writer.into_inner().map_err(|e| span.error(ErrorKind::Domain, format!("CSV {e}")))?;
    Ok(keyed::text(std::str::from_utf8(&bytes).unwrap()))
}

fn field(v: &Value, opts: &Options, fill: &Number, span: &Context<'_>) -> Result<String, Error> {
    let mut s = if let Some(s) = keyed::name(v) { if opts.trim { s.trim().to_owned() } else { s.to_string() } } else if let Value::Number(n) = v {
        if opts.common.values.contains_key("fill") && n.grade_order(fill).is_eq() { return Ok(String::new()); }
        if let Some(n) = n.as_integer() { n.to_string() } else if let Some(n) = n.as_float() {
            let s = n.to_string();
            if n.is_finite() && !s.contains(['.', 'e', 'E']) { format!("{s}.0") } else { s }
        } else if let Some(n) = n.as_exact().filter(|n| n.is_integer()) { n.numer().to_string() } else { return Err(span.error(ErrorKind::Domain, "CSV cells must be real floats, integers or strings")); }
    } else { return Err(span.error(ErrorKind::Domain, "CSV cells must be real floats, integers or strings")); };
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
