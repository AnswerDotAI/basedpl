//! Helpers that basedpl's test binaries share.
use basedpl::{Error, EvalOptions, Session, Value as AplValue};

/// The value of `code`, evaluated apart from the names of the session that the calling test holds, if any (`Session::eval_apart`).
pub fn run(code: &str) -> Result<Option<AplValue>, Error> {
    let result = Session::current().eval_apart(code, EvalOptions::default());
    match result.error { Some(e) => Err(e), None => Ok(result.value) }
}

pub fn number(n: impl TryInto<basedpl::Number>) -> AplValue { AplValue::number(n).unwrap() }
pub fn float(n: f64) -> AplValue { AplValue::Number(basedpl::Number::float(n, basedpl::buffer::FloatWidth::F64)) }
pub fn vector(values: &[f64]) -> AplValue { AplValue::from_parts(vec![values.len()], values.iter().copied().map(float).collect(), float(0.0)).unwrap() }

#[track_caller]
pub fn check_in(session: &Session, code: &str, expected: AplValue) { assert_same(evaluated(session, code), &expected); }

/// The value of `code`, which must evaluate without error.
#[track_caller]
pub fn evaluated(session: &Session, code: &str) -> Option<AplValue> {
    let result = session.eval(code);
    assert!(result.error.is_none(), "{code}: {:?}", result.error);
    result.value
}

/// Asserts that `actual` is exactly `expected`, as `Value::same` compares them.
#[track_caller]
pub fn assert_same(actual: impl Into<Option<AplValue>>, expected: &AplValue) {
    let actual = actual.into();
    assert!(actual.as_ref().is_some_and(|a| a.same(expected)), "{actual:?} is not {expected:?}");
}

/// Evaluates `expected` after `code`, under the settings that `code` leaves, in a module of its own.
#[track_caller]
pub fn equiv_in(session: &Session, code: &str, expected: &str) {
    let actual = evaluated(session, code);
    let wanted = session.eval_apart(expected, EvalOptions::default());
    assert!(wanted.error.is_none(), "{expected}: {:?}", wanted.error);
    assert_same(actual, &wanted.value.expect("expected an array"));
}

#[track_caller]
pub fn equiv(code: &str, expected: &str) { equiv_in(&Session::new(), code, expected); }

macro_rules! equiv_in {
    ($session:expr; $($code:expr => $expected:expr),* $(,)?) => {{
        let session = $session;
        $(equiv_in(session, $code, $expected);)*
    }};
}

#[track_caller]
pub fn fails_in(session: &Session, kind: &str, codes: &[&str]) {
    for code in codes {
        let result = session.eval(code);
        match result.error {
            Some(error) => assert_eq!(error.kind.name(), kind, "{code}: {error}"), None => panic!("{code}: expected {kind:?}, got {:?}", result.value)
        }
    }
}
