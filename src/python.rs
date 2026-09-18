use crate::{Array, Element, EvalOptions, Evaluation, InterruptHandle, Number, Session, Span};
use num_bigint::BigInt;
use num_rational::BigRational;
use pyo3::{
    exceptions::{PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
    sync::MutexExt,
    types::{PyComplex, PyComplexMethods, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
};
use std::{
    sync::{mpsc, Mutex},
    time::Duration,
};

fn import_array(raw: &Bound<'_, PyDict>, depth: usize) -> PyResult<Array> {
    if depth > 128 { return Err(PyValueError::new_err("array nesting exceeds 128 levels")); }
    let element = |o: Bound<'_, PyAny>| -> PyResult<Element> {
        if let Ok(a) = o.extract::<PyRef<'_, PyArray>>() { return Ok(Element::Nested(a.inner.clone())); }
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

#[pyclass(frozen, name = "_Array")]
struct PyArray { inner: Array }

#[pymethods]
impl PyArray {
    #[new]
    fn new(raw: &Bound<'_, PyDict>) -> PyResult<Self> { Ok(Self { inner: import_array(raw, 0)? }) }
    #[getter]
    fn shape(&self) -> Vec<usize> { self.inner.shape().to_vec() }
    fn parts(&self, py: Python<'_>) -> PyResult<Py<PyDict>> { array(py, &self.inner) }
    fn __repr__(&self) -> String { self.inner.to_string() }
}

struct Location { name: String, text: String, range: (usize, usize) }
impl From<&Span> for Location {
    fn from(s: &Span) -> Self { Self { name: s.source.name.clone(), text: s.source.text.clone(), range: (s.range.start, s.range.end) } }
}
struct Diagnostic {
    kind: String,
    message: String,
    display: String,
    location: Location,
    calls: Vec<Location>,
}
struct Reply { value: Option<Array>, output: Vec<String>, error: Option<Diagnostic> }
impl From<Evaluation> for Reply {
    fn from(e: Evaluation) -> Self {
        let error = e.error.map(|e| Diagnostic {
            kind: e.kind.to_string(),
            display: e.to_string(),
            location: (&e.span).into(),
            calls: e.calls.iter().map(Location::from).collect(),
            message: e.message,
        });
        Self { value: e.value, output: e.output, error }
    }
}
struct Request {
    code: Option<String>,
    function: Option<String>,
    args: Vec<Array>,
    bindings: Vec<(String, Array)>,
    options: EvalOptions,
    reply: mpsc::Sender<Result<Reply, &'static str>>,
}
impl Request {
    fn run(&mut self, session: &mut Session) -> Result<Reply, &'static str> {
        for (name, value) in self.bindings.drain(..) { session.set(&name, value).map_err(|_| "binding requires an ordinary APL name")?; }
        let options = std::mem::take(&mut self.options);
        Ok(match (&self.code, &self.function) {
            (Some(code), _) => session.eval_with(code, options),
            (_, Some(function)) => session.call_with(function, &self.args, options),
            _ => Evaluation::default(),
        }
        .into())
    }
}
struct WorkerState { sender: Option<mpsc::Sender<Request>>, active: Option<InterruptHandle> }

#[pyclass(frozen, name = "_Session")]
struct PySession { state: Mutex<WorkerState>, serial: Mutex<()> }

#[pymethods]
impl PySession {
    #[new]
    fn new() -> PyResult<Self> {
        let (sender, receiver) = mpsc::channel::<Request>();
        std::thread::Builder::new()
            .name("miniapl".into())
            .spawn(move || {
                let mut session = Session::new();
                for mut request in receiver {
                    let result = request.run(&mut session);
                    let _ = request.reply.send(result);
                }
            })
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { state: Mutex::new(WorkerState { sender: Some(sender), active: None }), serial: Mutex::new(()) })
    }
    #[pyo3(signature = (*, code=None, function=None, args=Vec::new(), bindings=Vec::new(), timeout=None, echo=false))]
    #[allow(clippy::too_many_arguments)]
    fn request(
        &self,
        py: Python<'_>,
        code: Option<String>,
        function: Option<String>,
        args: Vec<PyRef<'_, PyArray>>,
        bindings: Vec<(String, PyRef<'_, PyArray>)>,
        timeout: Option<f64>,
        echo: bool,
    ) -> PyResult<Py<PyDict>> {
        if code.is_some() && function.is_some() { return Err(PyValueError::new_err("choose code or function, not both")); }
        let options = options(timeout, echo)?;
        let interrupt = options.interrupt.clone();
        let (reply, mut receiver) = mpsc::channel();
        let request = Request {
            code,
            function,
            args: args.iter().map(|a| a.inner.clone()).collect(),
            bindings: bindings.into_iter().map(|(n, a)| (n, a.inner.clone())).collect(),
            options,
            reply,
        };
        let _serial = self.serial.lock_py_attached(py).unwrap();
        py.check_signals()?;
        {
            let mut state = self.state.lock().unwrap();
            state
                .sender
                .as_ref()
                .ok_or_else(|| PyRuntimeError::new_err("session is closed"))?
                .send(request)
                .map_err(|_| PyRuntimeError::new_err("session worker stopped"))?;
            state.active = Some(interrupt.clone());
        }
        let mut signal = None;
        let result = loop {
            let received = {
                let receiver = &mut receiver;
                py.detach(move || receiver.recv_timeout(Duration::from_millis(10)))
            };
            if signal.is_none() {
                if let Err(e) = py.check_signals() {
                    interrupt.interrupt();
                    signal = Some(e);
                }
            }
            match received {
                Ok(result) => break result.map_err(PyValueError::new_err),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break Err(PyRuntimeError::new_err("session worker stopped")),
            }
        };
        self.state.lock().unwrap().active = None;
        if let Some(e) = signal {
            if let Ok(result) = &result { e.value(py).setattr("output", &result.output)?; }
            return Err(e);
        }
        response(py, result?)
    }
    fn interrupt(&self) { if let Some(active) = &self.state.lock().unwrap().active { active.interrupt(); } }
    fn close(&self) {
        let mut state = self.state.lock().unwrap();
        if let Some(active) = &state.active { active.interrupt(); }
        state.sender = None;
    }
}
impl Drop for PySession { fn drop(&mut self) { self.close(); } }

fn options(timeout: Option<f64>, echo: bool) -> PyResult<crate::EvalOptions> {
    let timeout = timeout
        .map(|seconds| std::time::Duration::try_from_secs_f64(seconds).map_err(|_| PyValueError::new_err("timeout must be finite and nonnegative")))
        .transpose()?;
    Ok(crate::EvalOptions { timeout, echo, ..crate::EvalOptions::default() })
}

fn response(py: Python<'_>, result: Reply) -> PyResult<Py<PyDict>> {
    fn location(py: Python<'_>, location: &Location) -> PyResult<Py<PyDict>> {
        let source = PyDict::new(py);
        source.set_item("name", &location.name)?;
        source.set_item("text", &location.text)?;
        let d = PyDict::new(py);
        d.set_item("source", source)?;
        d.set_item("span", location.range)?;
        Ok(d.unbind())
    }
    let value = result.value.map(|inner| Py::new(py, PyArray { inner })).transpose()?;
    let error = result
        .error
        .as_ref()
        .map(|e| -> PyResult<Py<PyDict>> {
            let d = location(py, &e.location)?.into_bound(py);
            d.set_item("kind", &e.kind)?;
            d.set_item("message", &e.message)?;
            d.set_item("display", &e.display)?;
            let calls = PyList::empty(py);
            for span in &e.calls { calls.append(location(py, span)?)?; }
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
    m.add_class::<PyArray>()?;
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    m.add_function(wrap_pyfunction!(_check_reference, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("symbols", crate::editor::SYMBOLS.to_vec())?;
    Ok(())
}
