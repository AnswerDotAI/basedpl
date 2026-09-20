use crate::{
    array::generated_len,
    eval::Operand,
    execution::Context,
    primitive::{integer, numeric},
    Error, ErrorKind, Function, Value,
};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Call { Value(fn(Option<&Value>, &Value, &Context<'_>) -> Result<Value, Error>), Load }

#[derive(Clone, Copy, Debug)]
pub(crate) struct SystemFunction { pub name: &'static str, pub call: Call }

enum Builtin { Text(&'static str), Function(Call) }

const BUILTINS: &[(&str, Builtin)] = &[
    ("•A", Builtin::Text("ABCDEFGHIJKLMNOPQRSTUVWXYZ")),
    ("•D", Builtin::Text("0123456789")),
    ("•C", Builtin::Function(Call::Value(case_convert))),
    ("•UCS", Builtin::Function(Call::Value(unicode_convert))),
    ("•LOAD", Builtin::Function(Call::Load)),
    ("•SIGNAL", Builtin::Function(Call::Value(signal))),
];

pub(crate) fn lookup(name: &str) -> Option<Operand> {
    let &(name, ref builtin) = BUILTINS.iter().find(|(key, _)| key.eq_ignore_ascii_case(name))?;
    Some(match builtin {
        Builtin::Text(text) => Operand::Value(Value::new(vec![text.len()], text.chars().map(Value::Character).collect()).unwrap()),
        Builtin::Function(call) => Operand::Function(Function::system(SystemFunction { name, call: *call })),
    })
}

fn signal(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.is_some() { return Err(span.error(ErrorKind::Syntax, "•SIGNAL is monadic")); }
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "•SIGNAL needs an error name vector")); }
    let invalid = || span.error(ErrorKind::Domain, "•SIGNAL needs an ordinary error name such as 'DOMAIN ERROR'");
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
        Some(a) if a.is_singleton() => numeric(&a.at(0), span)?.integer().map_err(|k| span.error(k, "•C mode must be 1, ¯1 or ¯3"))?,
        _ => return Err(span.error(ErrorKind::Domain, "•C needs one case mode")),
    };
    if !matches!(mode, -3 | -1 | 1) { return Err(span.error(ErrorKind::Domain, "•C mode must be 1, ¯1 or ¯3")); }
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
        Value::from_parts(a.shape().to_vec(), data, item(a.prototype().clone())?).map_err(|k| span.error(k, "invalid case conversion"))
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
                if mode == 83 { return Err(span.error(ErrorKind::Unsupported, "•UCS signed bytes are out of scope")); }
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
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "encoded •UCS needs a vector")); }
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
    Value::from_parts(shape, data, if characters { integer(0) } else { Value::Character(' ') }).map_err(|k| span.error(k, "invalid Unicode result"))
}
