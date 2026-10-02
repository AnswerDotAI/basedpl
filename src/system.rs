use crate::{
    array::{generated_len, Items},
    eval::Operand,
    execution::Context,
    primitive::{integer, numeric, pervade, EmptyFill},
    Error, ErrorAt, ErrorKind, Function, Value,
};
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub(crate) enum Call {
    Value(fn(Option<&Value>, &Value, &Context<'_>) -> Result<Value, Error>),
    Session(fn(&mut crate::Session, Option<&Value>, &Value, &crate::Span) -> Result<Value, Error>),
    Regex(std::sync::Arc<::regex::Regex>, crate::regex::Operation),
    Distribution(std::sync::Arc<crate::distribution::Distribution>, crate::distribution::Operation),
    Generator(crate::distribution::Generator, crate::distribution::Draw),
    Load,
    Element(std::sync::Arc<str>),
    Mime,
    Time,
}

/// How many arguments a system function takes.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Valence { Monadic, Dyadic, Ambivalent }
use Valence::{Ambivalent, Dyadic, Monadic};

#[derive(Clone, Debug)]
pub(crate) struct SystemFunction { pub name: &'static str, pub call: Call, pub valence: Valence }

impl SystemFunction {
    /// A call with the wrong number of arguments is a SYNTAX error.
    pub(crate) fn check(&self, left: Option<&Value>, span: &crate::Span) -> Result<(), Error> {
        match (self.valence, left) {
            (Monadic, Some(_)) => Err(span.error(ErrorKind::Syntax, format!("{} is monadic", self.name))),
            (Dyadic, None) => Err(span.error(ErrorKind::Syntax, format!("{} needs a left argument", self.name))),
            _ => Ok(()),
        }
    }
}

/// A function value that calls `call` natively.
pub(crate) fn native(name: &'static str, call: Call, valence: Valence) -> Value { Value::Function(Function::system(SystemFunction { name, call, valence })) }

/// A keyed vector of native functions, each named by its key.
pub(crate) fn natives(entries: impl IntoIterator<Item = (&'static str, Call, Valence)>) -> Value {
    let (keys, functions) = entries.into_iter().map(|(name, call, valence)| (name.into(), native(name, call, valence))).unzip();
    crate::keyed::vector(keys, functions).expect("distinct keys")
}

enum Builtin { Text(&'static str), Function(Call, Valence) }

/// Help text for a system name, or the embedded glyph page that documents it.
enum Help { Text(&'static str), Page(&'static str) }

const BUILTINS: &[(&str, Builtin, Help)] = &[
    ("•a", Builtin::Text("ABCDEFGHIJKLMNOPQRSTUVWXYZ"), Help::Page("alphabet")),
    ("•d", Builtin::Text("0123456789"), Help::Page("digits")),
    ("•c", Builtin::Function(Call::Value(case_convert), Ambivalent), Help::Page("case")),
    ("•csv", Builtin::Function(Call::Value(crate::csv::parse), Ambivalent), Help::Text(CSV)),
    ("•tocsv", Builtin::Function(Call::Value(crate::csv::serialize), Ambivalent), Help::Text(TOCSV)),
    ("•json", Builtin::Function(Call::Value(crate::json::parse), Ambivalent), Help::Text(JSON)),
    ("•tojson", Builtin::Function(Call::Value(crate::json::serialize), Ambivalent), Help::Text(TOJSON)),
    ("•mime", Builtin::Function(Call::Mime, Monadic), Help::Text(MIME)),
    ("•element", Builtin::Function(Call::Value(crate::xml::factory), Monadic), Help::Text(ELEMENT)),
    ("•xml", Builtin::Function(Call::Value(crate::xml::serialize), Monadic), Help::Text(XML)),
    ("•svg", Builtin::Function(Call::Value(crate::xml::svg), Ambivalent), Help::Text(SVG)),
    ("•plot", Builtin::Function(Call::Value(crate::plot::plot), Ambivalent), Help::Text(PLOT)),
    ("•vfi", Builtin::Function(Call::Value(crate::data::vfi), Ambivalent), Help::Text(VFI)),
    ("•r", Builtin::Function(Call::Value(crate::regex::compile), Monadic), Help::Text(REGEX)),
    ("•normal", Builtin::Function(Call::Value(crate::distribution::normal), Monadic), Help::Text(DISTRIBUTION)),
    ("•uniform", Builtin::Function(Call::Value(crate::distribution::uniform), Monadic), Help::Text(DISTRIBUTION)),
    ("•beta", Builtin::Function(Call::Value(crate::distribution::beta), Monadic), Help::Text(DISTRIBUTION)),
    ("•bernoulli", Builtin::Function(Call::Value(crate::distribution::bernoulli), Monadic), Help::Text(DISTRIBUTION)),
    ("•binomial", Builtin::Function(Call::Value(crate::distribution::binomial), Monadic), Help::Text(DISTRIBUTION)),
    ("•cauchy", Builtin::Function(Call::Value(crate::distribution::cauchy), Monadic), Help::Text(DISTRIBUTION)),
    ("•chisquared", Builtin::Function(Call::Value(crate::distribution::chisquared), Monadic), Help::Text(DISTRIBUTION)),
    ("•exponential", Builtin::Function(Call::Value(crate::distribution::exponential), Monadic), Help::Text(DISTRIBUTION)),
    ("•fisher", Builtin::Function(Call::Value(crate::distribution::fisher), Monadic), Help::Text(DISTRIBUTION)),
    ("•gamma", Builtin::Function(Call::Value(crate::distribution::gamma), Monadic), Help::Text(DISTRIBUTION)),
    ("•inversegamma", Builtin::Function(Call::Value(crate::distribution::inversegamma), Monadic), Help::Text(DISTRIBUTION)),
    ("•laplace", Builtin::Function(Call::Value(crate::distribution::laplace), Monadic), Help::Text(DISTRIBUTION)),
    ("•lognormal", Builtin::Function(Call::Value(crate::distribution::lognormal), Monadic), Help::Text(DISTRIBUTION)),
    ("•logistic", Builtin::Function(Call::Value(crate::distribution::logistic_distribution), Monadic), Help::Text(DISTRIBUTION)),
    ("•poisson", Builtin::Function(Call::Value(crate::distribution::poisson), Monadic), Help::Text(DISTRIBUTION)),
    ("•student", Builtin::Function(Call::Value(crate::distribution::student), Monadic), Help::Text(DISTRIBUTION)),
    ("•weibull", Builtin::Function(Call::Value(crate::distribution::weibull), Monadic), Help::Text(DISTRIBUTION)),
    ("•rand", Builtin::Function(Call::Value(crate::distribution::generator), Monadic), Help::Text(RAND)),
    ("•nget", Builtin::Function(Call::Value(crate::data::read), Ambivalent), Help::Text(NGET)),
    ("•nput", Builtin::Function(Call::Value(crate::data::write), Dyadic), Help::Text(NPUT)),
    ("•ucs", Builtin::Function(Call::Value(unicode_convert), Ambivalent), Help::Page("unicode")),
    ("•load", Builtin::Function(Call::Load, Monadic), Help::Page("load")),
    ("•signal", Builtin::Function(Call::Value(signal), Monadic), Help::Page("error-guard")),
    ("•storage", Builtin::Function(Call::Value(storage), Monadic), Help::Text(STORAGE)),
    ("•time", Builtin::Function(Call::Time, Ambivalent), Help::Text(TIME)),
    ("•nc", Builtin::Function(Call::Session(crate::Session::system_nc), Monadic), Help::Text(NC)),
    ("•nl", Builtin::Function(Call::Session(crate::Session::system_nl), Ambivalent), Help::Text(NL)),
    ("•src", Builtin::Function(Call::Session(crate::Session::system_src), Monadic), Help::Text(SRC)),
    ("•ex", Builtin::Function(Call::Session(crate::Session::system_ex), Monadic), Help::Text(EX)),
];

const CSV: &str = r#"`•csv text` parses CSV into a vector of columns. Headers become keys. Numeric columns become numbers. Missing numeric cells become NaN. An integer column with missing cells stays exact. Missing text cells become `""`.

`X •csv text` takes options on the left: `header`, `separator`, `quotechar`, `doublequote`, `escapechar`, `decimal`, `thousands`, `trim`, `fill`, `text_columns`, `numeric_columns` and `missing`. The Files, CSV and JSON guide describes them.

Errors: DOMAIN for invalid options or duplicate headers; LENGTH for unequal record widths."#;

const TOCSV: &str = r#"`•tocsv T` returns CSV text for a vector of columns. Keys supply the header. Column lengths must agree.

`X •tocsv T` takes options on the left: `header`, `separator`, `quotechar`, `doublequote`, `escapechar`, `decimal`, `thousands`, `trim`, `fill`, `forcequotes` and `lineending`. NaN writes as an empty cell, and so does the exact value given as `fill`. `"forcequotes":2` quotes every field.

Errors: DOMAIN for nonintegral rationals, complex numbers, functions or nested cells; LENGTH for unequal columns."#;

const JSON: &str = r#"`•json text` parses JSON. Objects become keyed vectors. Arrays become vectors. Strings become character vectors. Integers stay exact. `true` and `false` become `1x` and `0x`.

`["fill":v] •json text` replaces `null` with `v`. The default is NaN. An integer array with a `null` stays exact.

Errors: DOMAIN for malformed JSON, with its line and column."#;

const TOJSON: &str = r#"`•tojson Y` returns JSON text. Keyed axes become objects. Unkeyed axes become arrays. Character vectors become strings. Keyed entries that hold functions, such as `_mime` renderers, are left out.

NaN exports as `null`. `["fill":v] •tojson Y` also writes `v` as `null`.

Errors: DOMAIN for an infinity that isn't `fill`, out-of-range floats, nonintegral rationals, complex numbers and other functions."#;

const VFI: &str = r"`•vfi text` returns `[valid numbers]` for the whitespace-separated fields of `text`. An invalid field has flag `0x` and value `0`. Fields are parsed as numbers, never executed.

`separators •vfi text` splits on each character in `separators` instead. It trims whitespace around fields. An empty field is a valid `0`.

Errors: DOMAIN when either argument is not text.";

const NGET: &str = r#"`•nget path` reads a UTF-8 text file. `•nget "-"` reads the rest of standard input.

`X •nget path` takes options on the left: `binary` (`1` reads a vector of byte values) and `encoding` (`"UTF-8"`).

Errors: VALUE for missing files, invalid UTF-8, other file errors and standard input that the frontend doesn't provide; DOMAIN for invalid options."#;

const NPUT: &str = r#"`path •nput data` writes `data` to a new UTF-8 file. It returns the number of bytes written.

`X •nput data` takes options on the left: `path`, `overwrite` (`1` replaces an existing file), `binary` (`1` writes a vector of byte values) and `encoding` (`"UTF-8"`). Plain text on the left is the path.

Errors: VALUE for an existing file without `overwrite`, and for other file errors; DOMAIN for invalid options or byte values, or `binary` with `encoding`; RANK for data that is not a vector."#;

const ELEMENT: &str = r#"`•element tag` returns an element function for XML tag `tag`. Call it with attributes on the left and children on the right, as in `["r":10] circle ""`. An empty right argument, `""` or `⍬`, gives no children. It returns a keyed vector with `tag`, `attrs` and `children` entries.

Errors: DOMAIN for invalid tag or attribute names."#;

const XML: &str = r#"`•xml tree` returns the XML text of an element tree. It escapes `&`, `<`, `>` and `"` in text and attribute values. A numeric vector attribute becomes space-separated numbers. Children are text, elements or vectors of children. An element with no children closes itself.

Errors: DOMAIN for invalid names or attribute values."#;

const SVG: &str = r#"`X •svg children` returns an `svg` element with attributes `X`. It adds `xmlns` for the SVG namespace and `viewBox="0 0 100 100"`. Attributes in `X` replace these. Its `_mime` field makes notebooks display it as a picture."#;

const MIME: &str = r"`•mime Y` returns the MIME bundle that display uses for `Y`. The bundle is a keyed vector from MIME types to text. It always has `text/plain`. If `Y` has a function in its `_mime` field, `•mime` calls it with `Y` as `⍵` and adds its entries.

Display shows the text form when a renderer fails. Only a direct `•mime` call reports the error.";

const PLOT: &str = r#"`X •plot Y` returns a plot spec: a keyed vector holding the data `Y`, the settings `X` and a renderer. Notebooks display the spec as an SVG chart. Plain text on the left is shorthand for `mark`.

The structure of `Y` chooses the series and axes:

- A vector plots its values against `0…n-1`.
- A keyed vector of numbers uses its keys as x labels.
- A matrix plots one series per row. Row keys name the series. Column keys label x.
- With the `cell` mark, each row of a matrix is a row of cells, coloured by value. Row keys label the rows, and row 0 is at the top.
- A table, a keyed vector of equal-length columns, plots each column as a series. A column named `x` supplies the x values.
- A vector or matrix of plots draws a figure.

| Setting | Holds | Default |
|---|---|---|
| `mark` | `"line"`, `"point"`, `"bar"` or `"cell"` | `"line"` |
| `title` | Chart title | none |
| `width`, `height` | Size in pixels | `600`, `400` |
| `x`, `y` | `title`, `scale` (`"linear"` or `"log"`), `ticks`, and `axis` (`$f` hides that axis) | |
| `legend` | `position` (`"end"` or a corner) and `border` | none |
| `grid` | `$f` hides the grid lines | `$t`, or `$f` for cells |
| `axes` | `$f` hides both axes, with their ticks and titles. An `axis` setting in `x` or `y` overrides it for that axis | `$t` |
| `flip` | `$t` swaps the axes | `$f` |
| `palette` | Colours for numbers: `"viridis"`, `"gray"`, or a list of colours | `"viridis"` |
| `colorbar` | `$t` shows the colour scale beside the plot | `$f` |
| `color`, `size`, `labels` | Styles for every series | |
| `series` | Styles for one series, keyed by its name | |
| `widths`, `heights`, `share` | Figure cell sizes and shared axis ranges | |

`color` also takes one number per point. `palette` turns these numbers into colours, over the range of every series in the plot. A cell takes its colour from its own value unless `color` is set.

`•mime` reports these errors: DOMAIN for unknown settings or values, and for cells mixed with other marks; LENGTH when series, colours, sizes or labels don't match the x values; RANK for data that isn't a vector, matrix or table."#;

const REGEX: &str = r"`•r pattern` compiles a Rust regex. It returns a keyed vector of functions that share the pattern: `match`, `position`, `length`, `groups` and `replace`. Positions count characters from 0.

`template p.replace text` expands `$0`, `$1`, `${name}` and `$$` in `template`. Flags go in the pattern, as in `(?i)`.

Errors: DOMAIN for invalid patterns, including look-around and backreferences, and for non-text arguments; SYNTAX for wrong valence; LIMIT for oversized results.";

const DISTRIBUTION: &str = r"A distribution constructor, such as `•normal 0 1`, returns a keyed vector of four functions:

- `sample shape` draws random values. `g sample shape` draws them from a generator made by `•rand`.
- `density x` gives the probability density, or the probability mass for a discrete distribution.
- `cdf x` gives P(X ≤ x).
- `quantile p` inverts the CDF.

Parameters are finite real units or vectors. Scale, shape, rate and degrees of freedom are positive, except where stated.

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

Errors: DOMAIN for invalid parameters, non-real inputs or p ∉ [0,1]; LENGTH for the wrong number of parameters; RANK for matrix parameters or shapes; SYNTAX for dyadic calls other than `sample`; LIMIT for oversized shapes or sampler ranges.";

const RAND: &str = r"`•rand seed` returns a generator: a keyed vector of two functions that draw from one stream of random numbers. The seed is a nonnegative integer. The same seed gives the same draws.

- `roll Y` works like `¿Y`.
- `X deal Y` works like `X¿Y`.

A distribution's `sample` takes a generator on its left, as in `g d.sample 3`. Copies of a generator share its stream. Drawing from one copy moves every copy on.

Errors: DOMAIN for a seed that is not a nonnegative integer, or a left argument to `sample` that is not a generator; LENGTH or RANK for more than one seed; SYNTAX for a dyadic call to `•rand`.";

const NC: &str = r"`•nc names` gives the class of each name: `¯1` invalid, `0` undefined, `2` value, `3` function, `4` operator. A character vector names one binding. An array of strings keeps its shape.";

const STORAGE: &str = r#"`•storage Y` names the storage that holds the items of `Y`: `"boolean"`, `"integer"`, `"float"`, `"complex"`, `"character"` or `"mixed"`. An atom gives its own kind, which can also be `"rational"` or `"function"`. Compact storage holds items of one kind. Mixed storage keeps each item's kind. Boxed display marks the same storage on its bottom edge."#;

const TIME: &str = r#"`•time t` gives the seconds since time `t`. Time counts from the session's start on a monotonic clock. `•time 0` is the current reading. `t←•time 0` starts a timer. `•time t` then gives the seconds since it started.

`F •time x` calls each function in `F` on `x` for about 0.1 s. The result is each function's fastest time per call in seconds, with the shape and keys of `F`. With `F←["sum":+/ "max":⌈/]`, `F •time x` labels each time. Time a dyadic function with its left argument bound, as in `2⍃⍴`.

Errors: DOMAIN for a left argument that holds anything but functions, or a `t` that isn't a number. Errors from a timed function stop the timing."#;

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
        Builtin::Text(text) => Operand::Value(crate::keyed::text(text)),
        Builtin::Function(call, valence) => Operand::Function(Function::system(SystemFunction { name, call: call.clone(), valence: *valence })),
    })
}

fn storage(_: Option<&Value>, right: &Value, _: &Context<'_>) -> Result<Value, Error> { Ok(crate::keyed::text(right.storage_name())) }

fn signal(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "•signal needs an error name vector")); }
    let invalid = || span.domain_error("•signal needs an ordinary error name such as \"DOMAIN ERROR\"");
    let kind = match crate::keyed::name(right).ok_or_else(invalid)?.as_ref() {
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
        Some(a) if a.is_singleton() => numeric(&a.at(0), span)?.integer().error_at(span, "•c mode must be 1, ¯1 or ¯3")?,
        _ => return Err(span.domain_error("•c needs one case mode")),
    };
    if !matches!(mode, -3 | -1 | 1) { return Err(span.domain_error("•c mode must be 1, ¯1 or ¯3")); }
    let mapper = icu_casemap::CaseMapper::new();
    let case = |e: Value| {
        Ok(match e {
            Value::Character(c) => Value::Character(match mode {
                1 => mapper.simple_uppercase(c),
                -1 => mapper.simple_lowercase(c),
                _ => mapper.simple_fold(c),
            }),
            e => e,
        })
    };
    pervade(right, &case, &EmptyFill::Mapped, span)
}

fn unicode_convert(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let invalid = || span.domain_error("invalid Unicode conversion");
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
        let text = crate::keyed::name(&name).filter(|_| !name.is_atom()).ok_or_else(invalid)?;
        if !matches!(&*text, "UTF-8" | "UTF-16" | "UTF-32") { return Err(invalid()); }
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "encoded •ucs needs a vector")); }
        Some(text)
    } else { None };
    let shape = |len: usize| {
        let shape = if encoding.is_some() { vec![len] } else { right.shape().to_vec() };
        generated_len(&shape).map(|_| shape).error_at(span, "Unicode result exceeds element limit")
    };
    let result = if matches!(right.prototype(), Value::Character(_)) {
        let chars: Cow<[char]> = match right.as_items() {
            Items::Characters(cs) => Cow::Borrowed(cs),
            Items::Values(vs) => Cow::Owned(vs.iter().map(|v| if let Value::Character(c) = v { Ok(*c) } else { Err(invalid()) }).collect::<Result<_, _>>()?),
            _ => return Err(invalid()),
        };
        let codes: Vec<i64> = match encoding.as_deref() {
            Some("UTF-8") => chars.iter().collect::<String>().bytes().map(i64::from).collect(),
            Some("UTF-16") => chars.iter().collect::<String>().encode_utf16().map(i64::from).collect(),
            _ => chars.iter().map(|&c| i64::from(u32::from(c))).collect(),
        };
        if encoding.is_none() && right.is_atom() { return Ok(integer(codes[0])); }
        Value::integers(shape(codes.len())?, codes)
    } else {
        if !matches!(right.prototype(), Value::Number(_)) { return Err(invalid()); }
        let codes = right.as_items().nonnegative_integers().map_err(|_| invalid())?;
        let chars: Vec<char> = match encoding.as_deref() {
            Some("UTF-8") => {
                let bytes: Vec<_> = codes.into_iter().map(u8::try_from).collect::<Result<_, _>>().map_err(|_| invalid())?;
                std::str::from_utf8(&bytes).map_err(|_| invalid())?.chars().collect()
            }
            Some("UTF-16") => {
                let units: Vec<_> = codes.into_iter().map(u16::try_from).collect::<Result<_, _>>().map_err(|_| invalid())?;
                char::decode_utf16(units).collect::<Result<_, _>>().map_err(|_| invalid())?
            }
            _ => codes.into_iter().map(|n| u32::try_from(n).ok().and_then(char::from_u32)).collect::<Option<_>>().ok_or_else(invalid)?,
        };
        if encoding.is_none() && right.is_atom() { return Ok(Value::Character(chars[0])); }
        Value::characters(shape(chars.len())?, chars)
    };
    result.and_then(|a| if encoding.is_none() { a.with_layout(right.layout().clone()) } else { Ok(a) }).error_at(span, "invalid Unicode result")
}
