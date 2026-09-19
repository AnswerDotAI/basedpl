use crate::{
    array::generated_len,
    eval::Operand,
    execution::Context,
    primitive::{integer, numeric},
    Array, Element, Error, ErrorKind, Function,
};

type Call = fn(Option<&Array>, &Array, &Context<'_>) -> Result<Array, Error>;

#[derive(Clone, Copy, Debug)]
pub(crate) struct SystemFunction { pub name: &'static str, pub call: Call }

enum Builtin { Text(&'static str), Function(Call) }

const BUILTINS: &[(&str, Builtin)] = &[
    ("•A", Builtin::Text("ABCDEFGHIJKLMNOPQRSTUVWXYZ")),
    ("•D", Builtin::Text("0123456789")),
    ("•C", Builtin::Function(case_convert)),
    ("•UCS", Builtin::Function(unicode_convert)),
];

pub(crate) fn lookup(name: &str) -> Option<Operand> {
    let &(name, ref builtin) = BUILTINS.iter().find(|(key, _)| key.eq_ignore_ascii_case(name))?;
    Some(match builtin {
        Builtin::Text(text) => Operand::Array(Array::new(vec![text.len()], text.chars().map(Element::Character).collect()).unwrap()),
        Builtin::Function(call) => Operand::Function(Function::system(SystemFunction { name, call: *call })),
    })
}

fn case_convert(left: Option<&Array>, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    let mode = match left {
        None => -3,
        Some(a) if a.is_singleton() => numeric(&a.at(0), span)?.integer().map_err(|k| span.error(k, "•C mode must be 1, ¯1 or ¯3"))?,
        _ => return Err(span.error(ErrorKind::Domain, "•C needs one case mode")),
    };
    if !matches!(mode, -3 | -1 | 1) { return Err(span.error(ErrorKind::Domain, "•C mode must be 1, ¯1 or ¯3")); }
    fn map(a: &Array, mode: isize, span: &Context<'_>) -> Result<Array, Error> {
        let mapper = icu_casemap::CaseMapper::new();
        let item = |e: Element| {
            span.check()?;
            Ok(match e {
                Element::Character(c) => Element::Character(match mode {
                    1 => mapper.simple_uppercase(c),
                    -1 => mapper.simple_lowercase(c),
                    _ => mapper.simple_fold(c),
                }),
                Element::Nested(a) => Element::Nested(map(&a, mode, span)?),
                e => e,
            })
        };
        let data = a.elements().map(item).collect::<Result<_, Error>>()?;
        Array::from_parts(a.shape().to_vec(), data, item(a.prototype().clone())?).map_err(|k| span.error(k, "invalid case conversion"))
    }
    map(right, mode, span)
}

fn unicode_convert(left: Option<&Array>, right: &Array, span: &Context<'_>) -> Result<Array, Error> {
    let invalid = || span.error(ErrorKind::Domain, "invalid Unicode conversion");
    let encoding = if let Some(spec) = left {
        let name = if matches!(spec.elements().next(), Some(Element::Nested(_))) {
            if spec.shape().len() > 1 || !(1..=2).contains(&spec.len()) { return Err(invalid()); }
            if spec.len() == 2 {
                let mode = numeric(&spec.at(1), span)?.integer().map_err(|_| invalid())?;
                if mode == 83 { return Err(span.error(ErrorKind::Unsupported, "•UCS signed bytes are out of scope")); }
                if mode != 0 { return Err(invalid()); }
            }
            spec.at(0).as_array()
        } else { spec.clone() };
        if name.shape().len() != 1 { return Err(invalid()); }
        let name: String = name
            .elements()
            .map(|e| match e { Element::Character(c) => Ok(c), _ => Err(invalid()) })
            .collect::<Result<_, _>>()?;
        if !matches!(name.as_str(), "UTF-8" | "UTF-16" | "UTF-32") { return Err(invalid()); }
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "encoded •UCS needs a vector")); }
        Some(name)
    } else { None };
    let characters = matches!(right.prototype(), Element::Character(_));
    let data = if characters {
        let mut data = Vec::new();
        for e in right.elements() {
            span.check()?;
            let Element::Character(c) = e else { return Err(invalid()); };
            match encoding.as_deref() {
                Some("UTF-8") => data.extend(c.encode_utf8(&mut [0; 4]).bytes().map(|b| integer(b as i64))),
                Some("UTF-16") => data.extend(c.encode_utf16(&mut [0; 2]).iter().map(|&u| integer(u as i64))),
                _ => data.push(integer(c as i64)),
            }
        }
        data
    } else {
        if !matches!(right.prototype(), Element::Number(_)) { return Err(invalid()); }
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
        chars.into_iter().map(Element::Character).collect()
    };
    let shape = if encoding.is_some() { vec![data.len()] } else { right.shape().to_vec() };
    generated_len(&shape).map_err(|k| span.error(k, "Unicode result exceeds element limit"))?;
    Array::from_parts(shape, data, if characters { integer(0) } else { Element::Character(' ') }).map_err(|k| span.error(k, "invalid Unicode result"))
}
