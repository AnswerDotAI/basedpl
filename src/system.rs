use crate::{
    array::generated_len,
    eval::Operand,
    execution::Context,
    primitive::{integer, numeric},
    Error, ErrorKind, Function, Value,
};

#[derive(Clone, Debug)]
pub(crate) enum Call {
    Value(fn(Option<&Value>, &Value, &Context<'_>) -> Result<Value, Error>),
    Session(fn(&mut crate::Session, Option<&Value>, &Value, &crate::Span) -> Result<Value, Error>),
    Regex(std::sync::Arc<::regex::Regex>, crate::regex::Operation),
    Distribution(std::sync::Arc<crate::distribution::Distribution>, crate::distribution::Operation),
    Load,
    Element(std::sync::Arc<str>),
    Mime,
}

#[derive(Clone, Debug)]
pub(crate) struct SystemFunction { pub name: &'static str, pub call: Call }

enum Builtin { Text(&'static str), Function(Call) }

/// Help text for a system name, or the embedded glyph page that documents it.
enum Help { Text(&'static str), Page(&'static str) }

const BUILTINS: &[(&str, Builtin, Help)] = &[
    ("•a", Builtin::Text("ABCDEFGHIJKLMNOPQRSTUVWXYZ"), Help::Page("alphabet")),
    ("•d", Builtin::Text("0123456789"), Help::Page("digits")),
    ("•c", Builtin::Function(Call::Value(case_convert)), Help::Page("case")),
    ("•csv", Builtin::Function(Call::Value(crate::csv::parse)), Help::Text(CSV)),
    ("•tocsv", Builtin::Function(Call::Value(crate::csv::serialize)), Help::Text(TOCSV)),
    ("•json", Builtin::Function(Call::Value(crate::json::parse)), Help::Text(JSON)),
    ("•tojson", Builtin::Function(Call::Value(crate::json::serialize)), Help::Text(TOJSON)),
    ("•mime", Builtin::Function(Call::Mime), Help::Text(MIME)),
    ("•element", Builtin::Function(Call::Value(crate::xml::factory)), Help::Text(ELEMENT)),
    ("•xml", Builtin::Function(Call::Value(crate::xml::serialize)), Help::Text(XML)),
    ("•svg", Builtin::Function(Call::Value(crate::xml::svg)), Help::Text(SVG)),
    ("•plot", Builtin::Function(Call::Value(crate::plot::plot)), Help::Text(PLOT)),
    ("•vfi", Builtin::Function(Call::Value(crate::data::vfi)), Help::Text(VFI)),
    ("•r", Builtin::Function(Call::Value(crate::regex::compile)), Help::Text(REGEX)),
    ("•normal", Builtin::Function(Call::Value(crate::distribution::normal)), Help::Text(DISTRIBUTION)),
    ("•uniform", Builtin::Function(Call::Value(crate::distribution::uniform)), Help::Text(DISTRIBUTION)),
    ("•beta", Builtin::Function(Call::Value(crate::distribution::beta)), Help::Text(DISTRIBUTION)),
    ("•bernoulli", Builtin::Function(Call::Value(crate::distribution::bernoulli)), Help::Text(DISTRIBUTION)),
    ("•binomial", Builtin::Function(Call::Value(crate::distribution::binomial)), Help::Text(DISTRIBUTION)),
    ("•cauchy", Builtin::Function(Call::Value(crate::distribution::cauchy)), Help::Text(DISTRIBUTION)),
    ("•chisquared", Builtin::Function(Call::Value(crate::distribution::chisquared)), Help::Text(DISTRIBUTION)),
    ("•exponential", Builtin::Function(Call::Value(crate::distribution::exponential)), Help::Text(DISTRIBUTION)),
    ("•fisher", Builtin::Function(Call::Value(crate::distribution::fisher)), Help::Text(DISTRIBUTION)),
    ("•gamma", Builtin::Function(Call::Value(crate::distribution::gamma)), Help::Text(DISTRIBUTION)),
    ("•inversegamma", Builtin::Function(Call::Value(crate::distribution::inversegamma)), Help::Text(DISTRIBUTION)),
    ("•laplace", Builtin::Function(Call::Value(crate::distribution::laplace)), Help::Text(DISTRIBUTION)),
    ("•lognormal", Builtin::Function(Call::Value(crate::distribution::lognormal)), Help::Text(DISTRIBUTION)),
    ("•logistic", Builtin::Function(Call::Value(crate::distribution::logistic_distribution)), Help::Text(DISTRIBUTION)),
    ("•poisson", Builtin::Function(Call::Value(crate::distribution::poisson)), Help::Text(DISTRIBUTION)),
    ("•student", Builtin::Function(Call::Value(crate::distribution::student)), Help::Text(DISTRIBUTION)),
    ("•weibull", Builtin::Function(Call::Value(crate::distribution::weibull)), Help::Text(DISTRIBUTION)),
    ("•nget", Builtin::Function(Call::Value(crate::data::read)), Help::Text(NGET)),
    ("•nput", Builtin::Function(Call::Value(crate::data::write)), Help::Text(NPUT)),
    ("•ucs", Builtin::Function(Call::Value(unicode_convert)), Help::Page("unicode")),
    ("•load", Builtin::Function(Call::Load), Help::Page("load")),
    ("•signal", Builtin::Function(Call::Value(signal)), Help::Page("error-guard")),
    ("•nc", Builtin::Function(Call::Session(crate::Session::system_nc)), Help::Text(NC)),
    ("•nl", Builtin::Function(Call::Session(crate::Session::system_nl)), Help::Text(NL)),
    ("•src", Builtin::Function(Call::Session(crate::Session::system_src)), Help::Text(SRC)),
    ("•ex", Builtin::Function(Call::Session(crate::Session::system_ex)), Help::Text(EX)),
];

const CSV: &str = r"`•csv text` parses CSV into a vector of columns. Headers become keys. Numeric columns become numbers. Missing numeric cells become `∞`. Missing text cells become `''`.

`X •csv text` takes options on the left: `header`, `separator`, `quotechar`, `doublequote`, `escapechar`, `decimal`, `thousands`, `trim`, `fill`, `text_columns`, `numeric_columns` and `missing`. The Files, CSV and JSON guide describes them.

Errors: DOMAIN for invalid options or duplicate headers; LENGTH for unequal record widths.";

const TOCSV: &str = r"`•tocsv T` returns CSV text for a vector of columns. Keys supply the header. Column lengths must agree.

`X •tocsv T` takes options on the left: `header`, `separator`, `quotechar`, `doublequote`, `escapechar`, `decimal`, `thousands`, `trim`, `fill`, `forcequotes` and `lineending`. `fill` writes that exact value as an empty cell. `'forcequotes':2` quotes every field.

Errors: DOMAIN for nonintegral rationals, complex numbers, functions or nested cells; LENGTH for unequal columns.";

const JSON: &str = r"`•json text` parses JSON. Objects become keyed vectors. Arrays become vectors. Strings become character vectors. Integers stay exact. `true` and `false` become `1x` and `0x`.

`('fill':v) •json text` replaces `null` with `v`. The default is `∞`.

Errors: DOMAIN for malformed JSON, with its line and column.";

const TOJSON: &str = r"`•tojson Y` returns JSON text. Keyed axes become objects. Unkeyed axes become arrays. Character vectors become strings. Keyed entries that hold functions, such as `_mime_` renderers, are left out.

`('fill':v) •tojson Y` writes `v` as `null`.

Errors: DOMAIN for infinity without `fill`, out-of-range floats, nonintegral rationals, complex numbers and other functions.";

const VFI: &str = r"`•vfi text` returns `(valid ⋄ numbers)` for the whitespace-separated fields of `text`. An invalid field has flag `0x` and value `0`. Fields are parsed as numbers, never executed.

`separators •vfi text` splits on each character in `separators` instead. It trims whitespace around fields. An empty field is a valid `0`.

Errors: DOMAIN when either argument is not text.";

const NGET: &str = r"`•nget path` reads a UTF-8 text file.

`X •nget path` takes options on the left: `binary` (`1` reads a vector of byte values) and `encoding` (`'UTF-8'`).

Errors: VALUE for missing files, invalid UTF-8 and other file errors; DOMAIN for invalid options.";

const NPUT: &str = r"`path •nput data` writes `data` to a new UTF-8 file. It returns the number of bytes written.

`X •nput data` takes options on the left: `path`, `overwrite` (`1` replaces an existing file), `binary` (`1` writes a vector of byte values) and `encoding` (`'UTF-8'`). Plain text on the left is the path.

Errors: VALUE for an existing file without `overwrite`, and for other file errors; DOMAIN for invalid options or byte values, or `binary` with `encoding`; RANK for data that is not a vector.";

const ELEMENT: &str = r"`•element tag` returns an element function for XML tag `tag`. Call it with attributes on the left and children on the right, as in `('r':10) circle ''`. An empty right argument, `''` or `⍬`, gives no children. It returns a keyed vector with `tag`, `attrs` and `children` entries.

Errors: DOMAIN for invalid tag or attribute names.";

const XML: &str = r#"`•xml tree` returns the XML text of an element tree. It escapes `&`, `<`, `>` and `"` in text and attribute values. A numeric vector attribute becomes space-separated numbers. Children are text, elements or vectors of children. An element with no children closes itself.

Errors: DOMAIN for invalid names or attribute values."#;

const SVG: &str = r#"`X •svg children` returns an `svg` element with attributes `X`. It adds `xmlns` for the SVG namespace and `viewBox="0 0 100 100"`. Attributes in `X` replace these. Its `_mime_` field makes notebooks display it as a picture."#;

const MIME: &str = r"`•mime Y` returns the MIME bundle that display uses for `Y`. The bundle is a keyed vector from MIME types to text. It always has `text/plain`. If `Y` has a function in its `_mime_` field, `•mime` calls it with `Y` as `⍵` and adds its entries.

Display shows the text form when a renderer fails. Only a direct `•mime` call reports the error.";

const PLOT: &str = r"`X •plot Y` returns a plot spec: a keyed vector holding the data `Y`, the settings `X` and a renderer. Notebooks display the spec as an SVG chart. Plain text on the left is shorthand for `mark`.

The structure of `Y` chooses the series and axes:

- A vector plots its values against `1…n`.
- A keyed vector of numbers uses its keys as x labels.
- A matrix plots one series per row. Row keys name the series. Column keys label x.
- A table, a keyed vector of equal-length columns, plots each column as a series. A column named `x` supplies the x values.
- A vector or matrix of plots draws a figure.

| Setting | Holds | Default |
|---|---|---|
| `mark` | `'line'`, `'point'` or `'bar'` | `'line'` |
| `title` | Chart title | none |
| `width`, `height` | Size in pixels | `600`, `400` |
| `x`, `y` | `title`, `scale` (`'linear'` or `'log'`) and `ticks` | |
| `legend` | `position` (`'end'` or a corner) and `border` | none |
| `grid` | `0` hides the grid lines | `1` |
| `flip` | `1` swaps the axes | `0` |
| `color`, `size`, `labels` | Styles for every series | |
| `series` | Styles for one series, keyed by its name | |
| `widths`, `heights`, `share` | Figure cell sizes and shared axis ranges | |

`•mime` reports these errors: DOMAIN for unknown settings or values; LENGTH when series, sizes or labels don't match the x values; RANK for data that isn't a vector, matrix or table.";

const REGEX: &str = r"`•r pattern` compiles a Rust regex. It returns a keyed vector of functions that share the pattern: `match`, `position`, `length`, `groups` and `replace`. Positions are 1-origin character indices.

`template p.replace text` expands `$0`, `$1`, `${name}` and `$$` in `template`. Flags go in the pattern, as in `(?i)`.

Errors: DOMAIN for invalid patterns, including look-around and backreferences, and for non-text arguments; SYNTAX for wrong valence; LIMIT for oversized results.";

const DISTRIBUTION: &str = r"A distribution constructor, such as `•normal 0 1`, returns a keyed vector of four monadic functions:

- `sample shape` draws random values.
- `density x` gives the probability density, or the probability mass for a discrete distribution.
- `cdf x` gives P(X ≤ x).
- `quantile p` inverts the CDF.

Parameters are finite real scalars or vectors. Scale, shape, rate and degrees of freedom are positive, except where stated.

| Constructor | Parameters |
|---|---|
| `•normal` | μ σ: mean, standard deviation |
| `•uniform` | a b: lower and upper bounds, a < b |
| `•bernoulli` | p ∈ [0,1] |
| `•binomial` | n p: integer trials n ≥ 0, p ∈ [0,1] |
| `•poisson` | λ ≥ 0: mean |
| `•beta` | α β: shapes |
| `•gamma` | k θ: shape, scale, with mean kθ |
| `•inversegamma` | α β: shape, scale, with density ∝ x⁻⁽ᵅ⁺¹⁾ exp(−β/x) |
| `•exponential` | λ: rate, with mean 1/λ |
| `•chisquared` | ν: degrees of freedom |
| `•student` | ν: degrees of freedom, with location 0 and scale 1 |
| `•fisher` | ν₁ ν₂: degrees of freedom |
| `•cauchy`, `•laplace`, `•logistic` | location, scale |
| `•lognormal` | μ σ: mean and standard deviation of log(X) |
| `•weibull` | k λ: shape, scale |

Errors: DOMAIN for invalid parameters, non-real inputs or p ∉ [0,1]; LENGTH for the wrong number of parameters; RANK for matrix parameters or shapes; SYNTAX for dyadic calls; LIMIT for oversized shapes or sampler ranges.";

const NC: &str = r"`•nc names` gives the class of each name: `¯1` invalid, `0` undefined, `2` value, `3` function or hybrid, `4` operator. A character vector names one binding. An array of strings keeps its shape.";

const NL: &str = r"`prefix •nl classes` lists the visible user names in `classes` that begin with `prefix`, as a sorted vector of strings. `•nl classes` lists them all.

Errors: DOMAIN for unsupported classes; RANK for a class matrix.";

const SRC: &str = r"`•src name` returns the definition text of a function or operator, including comments.

Errors: VALUE for an undefined name; DOMAIN for an array.";

const EX: &str = r"`•ex names` erases the nearest binding of each name. An outer binding can then become visible. It returns `1` when the name is gone. It returns `0` for an invalid or protected name.";

pub(crate) fn names() -> impl Iterator<Item = &'static str> { BUILTINS.iter().map(|(name, ..)| *name) }

/// The help for system name `name`, ignoring case.
pub(crate) fn help(name: &str) -> Option<&'static str> {
    match BUILTINS.iter().find(|(key, ..)| key.eq_ignore_ascii_case(name))? {
        (_, _, Help::Text(text)) => Some(text),
        (_, _, Help::Page(page)) => crate::inspection::page(page),
    }
}

pub(crate) fn lookup(name: &str) -> Option<Operand> {
    let &(name, ref builtin, _) = BUILTINS.iter().find(|(key, ..)| key.eq_ignore_ascii_case(name))?;
    Some(match builtin {
        Builtin::Text(text) => Operand::Value(Value::new(vec![text.len()], text.chars().map(Value::Character).collect()).unwrap()),
        Builtin::Function(call) => Operand::Function(Function::system(SystemFunction { name, call: call.clone() })),
    })
}

fn signal(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.is_some() { return Err(span.error(ErrorKind::Syntax, "•signal is monadic")); }
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "•signal needs an error name vector")); }
    let invalid = || span.error(ErrorKind::Domain, "•signal needs an ordinary error name such as 'DOMAIN ERROR'");
    let name: String = right
        .elements()
        .map(|e| match e { Value::Character(c) => Ok(c), _ => Err(invalid()) })
        .collect::<Result<_, _>>()?;
    let kind = match name.as_str() {
        "SYNTAX ERROR" => ErrorKind::Syntax,
        "INDEX ERROR" => ErrorKind::Index,
        "RANK ERROR" => ErrorKind::Rank,
        "LENGTH ERROR" => ErrorKind::Length,
        "VALUE ERROR" => ErrorKind::Value,
        "LIMIT ERROR" => ErrorKind::Limit,
        "DOMAIN ERROR" => ErrorKind::Domain,
        _ => return Err(invalid()),
    };
    Err(span.error(kind, "explicitly signalled"))
}

fn case_convert(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mode = match left {
        None => -3,
        Some(a) if a.is_singleton() => numeric(&a.at(0), span)?.integer().map_err(|k| span.error(k, "•c mode must be 1, ¯1 or ¯3"))?,
        _ => return Err(span.error(ErrorKind::Domain, "•c needs one case mode")),
    };
    if !matches!(mode, -3 | -1 | 1) { return Err(span.error(ErrorKind::Domain, "•c mode must be 1, ¯1 or ¯3")); }
    fn map(a: &Value, mode: isize, span: &Context<'_>) -> Result<Value, Error> {
        let mapper = icu_casemap::CaseMapper::new();
        let item = |e: Value| {
            span.check()?;
            Ok(match e {
                Value::Character(c) => Value::Character(match mode {
                    1 => mapper.simple_uppercase(c),
                    -1 => mapper.simple_lowercase(c),
                    _ => mapper.simple_fold(c),
                }),
                a @ Value::Array(_) => map(&a, mode, span)?,
                e => e,
            })
        };
        if a.is_atom() { return item(a.clone()); }
        let data = a.elements().map(item).collect::<Result<_, Error>>()?;
        a.layout().collect(data, item(a.prototype())?).map_err(|k| span.error(k, "invalid case conversion"))
    }
    map(right, mode, span)
}

fn unicode_convert(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let invalid = || span.error(ErrorKind::Domain, "invalid Unicode conversion");
    let encoding = if let Some(spec) = left {
        let name = if matches!(spec.elements().next(), Some(Value::Array(_))) {
            if spec.shape().len() > 1 || !(1..=2).contains(&spec.len()) { return Err(invalid()); }
            if spec.len() == 2 {
                let mode = numeric(&spec.at(1), span)?.integer().map_err(|_| invalid())?;
                if mode == 83 { return Err(span.error(ErrorKind::Unsupported, "•ucs signed bytes are out of scope")); }
                if mode != 0 { return Err(invalid()); }
            }
            spec.at(0).clone()
        } else { spec.clone() };
        if name.shape().len() != 1 { return Err(invalid()); }
        let name: String = name
            .elements()
            .map(|e| match e { Value::Character(c) => Ok(c), _ => Err(invalid()) })
            .collect::<Result<_, _>>()?;
        if !matches!(name.as_str(), "UTF-8" | "UTF-16" | "UTF-32") { return Err(invalid()); }
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "encoded •ucs needs a vector")); }
        Some(name)
    } else { None };
    let characters = matches!(right.prototype(), Value::Character(_));
    let data = if characters {
        let mut data = Vec::new();
        for e in right.elements() {
            span.check()?;
            let Value::Character(c) = e else { return Err(invalid()); };
            match encoding.as_deref() {
                Some("UTF-8") => data.extend(c.encode_utf8(&mut [0; 4]).bytes().map(|b| integer(b as i64))),
                Some("UTF-16") => data.extend(c.encode_utf16(&mut [0; 2]).iter().map(|&u| integer(u as i64))),
                _ => data.push(integer(c as i64)),
            }
        }
        data
    } else {
        if !matches!(right.prototype(), Value::Number(_)) { return Err(invalid()); }
        let codes: Vec<u32> = right
            .elements()
            .map(|e| {
                span.check()?;
                let n = numeric(&e, span)?.nonnegative_integer().map_err(|_| invalid())?;
                u32::try_from(n).map_err(|_| invalid())
            })
            .collect::<Result<_, Error>>()?;
        let chars: Vec<char> = match encoding.as_deref() {
            Some("UTF-8") => {
                let bytes: Vec<_> = codes.into_iter().map(u8::try_from).collect::<Result<_, _>>().map_err(|_| invalid())?;
                std::str::from_utf8(&bytes).map_err(|_| invalid())?.chars().collect()
            }
            Some("UTF-16") => {
                let units: Vec<_> = codes.into_iter().map(u16::try_from).collect::<Result<_, _>>().map_err(|_| invalid())?;
                char::decode_utf16(units).collect::<Result<_, _>>().map_err(|_| invalid())?
            }
            _ => codes.into_iter().map(char::from_u32).collect::<Option<_>>().ok_or_else(invalid)?,
        };
        chars.into_iter().map(Value::Character).collect()
    };
    if encoding.is_none() && right.is_atom() { return Ok(data[0].clone()); }
    let shape = if encoding.is_some() { vec![data.len()] } else { right.shape().to_vec() };
    generated_len(&shape).map_err(|k| span.error(k, "Unicode result exceeds element limit"))?;
    let result = Value::from_parts(shape, data, if characters { integer(0) } else { Value::Character(' ') });
    result.and_then(|a| if encoding.is_none() { a.with_layout(right.layout().clone()) } else { Ok(a) }).map_err(|k| span.error(k, "invalid Unicode result"))
}
