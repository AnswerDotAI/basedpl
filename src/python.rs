use crate::{Array, Element, Number, Session};
use num_bigint::BigInt;
use num_rational::BigRational;
use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    types::{PyComplex, PyComplexMethods, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
};

fn import_array(raw: &Bound<'_, PyDict>, depth: usize) -> PyResult<Array> {
    if depth > 128 { return Err(PyValueError::new_err("array nesting exceeds 128 levels")); }
    let element = |o: Bound<'_, PyAny>| -> PyResult<Element> {
        if let Ok(d) = o.cast::<PyDict>() { return Ok(Element::Nested(import_array(d, depth + 1)?)); }
        if let Ok(s) = o.cast::<PyString>() {
            let s = s.to_str()?;
            let mut chars = s.chars();
            let c = chars.next().ok_or_else(|| PyValueError::new_err("expected one character"))?;
            if chars.next().is_some() { return Err(PyValueError::new_err("expected one character")); }
            return Ok(Element::Character(c));
        }
        let number = if o.is_instance_of::<PyFloat>() { Number::try_from(o.extract::<f64>()?) } else if let Ok(z) = o.cast::<PyComplex>() { Number::try_from(num_complex::Complex64::new(z.real(), z.imag())) } else if o.is_instance_of::<PyInt>() { Number::try_from(BigRational::from_integer(o.extract::<BigInt>()?)) } else if o.is_instance_of::<PyTuple>() {
            let (n, d) = o.extract::<(BigInt, BigInt)>()?;
            Number::try_from(BigRational::new_raw(n, d))
        } else { return Err(PyTypeError::new_err("unsupported APL element")); };
        number.map(Element::Number).map_err(|k| PyValueError::new_err(k.to_string()))
    };
    let field = |key| raw.get_item(key)?.ok_or_else(|| PyValueError::new_err("missing array field"));
    let shape = field("shape")?.extract::<Vec<usize>>()?;
    let data = field("data")?.try_iter()?.map(|o| element(o?)).collect::<PyResult<Vec<_>>>()?;
    let prototype = element(field("prototype")?)?;
    Array::from_parts(shape, data, prototype).map_err(|k| PyValueError::new_err(k.to_string()))
}

fn array(py: Python<'_>, a: &Array) -> PyResult<Py<PyDict>> {
    fn element(py: Python<'_>, e: &Element) -> PyResult<Py<PyAny>> {
        Ok(match e {
            Element::Number(n) => {
                if let Some(n) = n.as_integer() { n.into_pyobject(py)?.into_any().unbind() } else if let Some(n) = n.as_exact() {
                    if n.is_integer() { n.numer().into_pyobject(py)?.into_any().unbind() } else { (n.numer(), n.denom()).into_pyobject(py)?.into_any().unbind() }
                } else if let Some(n) = n.as_complex() { PyComplex::from_doubles(py, n.re, n.im).into_any().unbind() } else { PyFloat::new(py, n.as_float().unwrap()).into_any().unbind() }
            }
            Element::Character(c) => PyString::new(py, &c.to_string()).into_any().unbind(),
            Element::Nested(a) => array(py, a)?.into_any(),
        })
    }
    let result = PyDict::new(py);
    let data = PyList::empty(py);
    for e in a.elements() { data.append(element(py, &e)?)?; }
    result.set_item("shape", a.shape())?;
    result.set_item("data", data)?;
    result.set_item("prototype", element(py, a.prototype())?)?;
    Ok(result.unbind())
}

// Access AND destruction are confined to the creating thread. Python's wrapper
// checks public calls; PyO3 also guards the private native object. No unsafe Send.
#[pyclass(unsendable, name = "_Session")]
struct PySession { inner: Session }

#[pymethods]
impl PySession {
    #[new]
    fn new() -> Self { Self { inner: Session::new() } }
    fn set(&mut self, name: &str, value: &Bound<'_, PyDict>) -> PyResult<()> {
        self.inner.set(name, import_array(value, 0)?).map_err(|_| PyValueError::new_err("binding requires an ordinary APL name"))
    }
    #[pyo3(signature = (code, timeout=None))]
    fn eval(&mut self, py: Python<'_>, code: &str, timeout: Option<f64>) -> PyResult<Py<PyDict>> {
        let result = match timeout {
            Some(seconds) => self.inner.eval_timeout(
                code,
                std::time::Duration::try_from_secs_f64(seconds).map_err(|_| PyValueError::new_err("timeout must be finite and nonnegative"))?,
            ),
            None => self.inner.eval(code),
        };
        let value = result.value.as_ref().map(|a| array(py, a)).transpose()?;
        let error = result
            .error
            .as_ref()
            .map(|e| -> PyResult<Py<PyDict>> {
                let d = PyDict::new(py);
                d.set_item("kind", e.kind.to_string())?;
                d.set_item("message", &e.message)?;
                d.set_item("source_name", &e.span.source.name)?;
                d.set_item("source", &e.span.source.text)?;
                d.set_item("span", (e.span.range.start, e.span.range.end))?;
                d.set_item("display", e.to_string())?;
                let calls = PyList::empty(py);
                for span in &e.calls {
                    let call = PyDict::new(py);
                    call.set_item("source_name", &span.source.name)?;
                    call.set_item("source", &span.source.text)?;
                    call.set_item("span", (span.range.start, span.range.end))?;
                    calls.append(call)?;
                }
                d.set_item("calls", calls)?;
                Ok(d.unbind())
            })
            .transpose()?;
        let d = PyDict::new(py);
        d.set_item("value", value)?;
        d.set_item("output", result.output)?;
        d.set_item("error", error)?;
        Ok(d.unbind())
    }
}

#[pyfunction]
fn run_cli(args: Vec<String>) -> i32 { crate::cli::run(&args) }

#[pyfunction]
fn _check_reference(case: &str, timeout: f64) -> PyResult<String> {
    let case = serde_json::from_str(case).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let timeout = std::time::Duration::try_from_secs_f64(timeout).map_err(|_| PyValueError::new_err("invalid timeout"))?;
    Ok(crate::reference::check(&case, crate::EvalOptions { timeout: Some(timeout), ..crate::EvalOptions::default() }).to_string())
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySession>()?;
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    m.add_function(wrap_pyfunction!(_check_reference, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
