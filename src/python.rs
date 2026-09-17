use crate::{Array, Element, Session};
use pyo3::{
    prelude::*,
    types::{PyComplex, PyDict, PyFloat, PyList, PyString},
};

fn array(py: Python<'_>, a: &Array) -> PyResult<Py<PyDict>> {
    fn element(py: Python<'_>, e: &Element) -> PyResult<Py<PyAny>> {
        Ok(match e {
            Element::Number(n) => {
                if let Some(n) = n.as_exact() {
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
    fn eval(&mut self, py: Python<'_>, code: &str) -> PyResult<Py<PyDict>> {
        let result = self.inner.eval(code);
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

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySession>()?;
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
