use crate::{eval::Operand, EvalOptions, Evaluation, Function, InterruptHandle, Number, Session, Source, Span, Value};
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

fn import_array(raw: &Bound<'_, PyDict>, depth: usize) -> PyResult<Value> {
    if depth > 128 { return Err(PyValueError::new_err("array nesting exceeds 128 levels")); }
    let element = |o: Bound<'_, PyAny>| -> PyResult<Value> {
        if let Ok(a) = o.extract::<PyRef<'_, PyArray>>() { return Ok(a.inner.clone()); }
        if let Ok(f) = o.extract::<PyRef<'_, PyFunction>>() { return Ok(Value::Function(f.inner.clone())); }
        if let Ok(d) = o.cast::<PyDict>() { return import_array(d, depth + 1); }
        if let Ok(s) = o.cast::<PyString>() {
            let s = s.to_str()?;
            let mut chars = s.chars();
            let c = chars.next().ok_or_else(|| PyValueError::new_err("expected one character"))?;
            if chars.next().is_some() { return Err(PyValueError::new_err("expected one character")); }
            return Ok(Value::Character(c));
        }
        let number = if o.is_instance_of::<PyFloat>() { Number::try_from(o.extract::<f64>()?) } else if let Ok(z) = o.cast::<PyComplex>() { Number::try_from(num_complex::Complex64::new(z.real(), z.imag())) } else if o.is_instance_of::<PyInt>() { Number::try_from(BigRational::from_integer(o.extract::<BigInt>()?)) } else if o.is_instance_of::<PyTuple>() {
            let (n, d) = o.extract::<(BigInt, BigInt)>()?;
            Number::try_from(BigRational::new_raw(n, d))
        } else { return Err(PyTypeError::new_err("unsupported APL element")); };
        number.map(Value::Number).map_err(|k| PyValueError::new_err(k.to_string()))
    };
    let field = |key| raw.get_item(key)?.ok_or_else(|| PyValueError::new_err("missing array field"));
    if let Some(atom) = raw.get_item("atom")? { return element(atom); }
    let shape = field("shape")?.extract::<Vec<usize>>()?;
    let data = field("data")?.try_iter()?.map(|o| element(o?)).collect::<PyResult<Vec<_>>>()?;
    let prototype = element(field("prototype")?)?;
    let mut result = Value::from_parts(shape, data, prototype).map_err(|k| PyValueError::new_err(k.to_string()))?;
    if let Some(names) = raw.get_item("axis_names")? {
        let names = names.extract::<Vec<Option<String>>>()?.into_iter().map(|n| n.map(Into::into)).collect();
        result = result.with_axis_names(names).map_err(|k| PyValueError::new_err(k.to_string()))?;
    }
    let Some(keys) = raw.get_item("axis_keys")? else { return Ok(result); };
    let keys = keys
        .extract::<Vec<Option<Vec<String>>>>()?
        .into_iter()
        .map(|k| k.map(|names| crate::keyed::Keys::new(names.into_iter().map(Into::into).collect())).transpose())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|k| PyValueError::new_err(k.to_string()))?;
    if keys.len() != result.shape().len() { return Err(PyValueError::new_err("axis_keys must have one entry per axis")); }
    result.with_keys(keys).map_err(|k| PyValueError::new_err(k.to_string()))
}

fn array(py: Python<'_>, a: &Value) -> PyResult<Py<PyDict>> {
    fn element(py: Python<'_>, e: &Value) -> PyResult<Py<PyAny>> {
        Ok(match e {
            Value::Number(n) => {
                if let Some(n) = n.as_integer() { n.into_pyobject(py)?.into_any().unbind() } else if let Some(n) = n.as_exact() {
                    if n.is_integer() { n.numer().into_pyobject(py)?.into_any().unbind() } else { (n.numer(), n.denom()).into_pyobject(py)?.into_any().unbind() }
                } else if let Some(n) = n.as_complex() { PyComplex::from_doubles(py, n.re, n.im).into_any().unbind() } else { PyFloat::new(py, n.as_float().unwrap()).into_any().unbind() }
            }
            Value::Character(c) => PyString::new(py, &c.to_string()).into_any().unbind(),
            a @ Value::Array(_) => array(py, a)?.into_any(),
            Value::Function(f) => Py::new(py, PyFunction { inner: f.clone() })?.into_any(),
        })
    }
    let result = PyDict::new(py);
    if a.is_atom() {
        result.set_item("atom", element(py, a)?)?;
        return Ok(result.unbind());
    }
    let data = PyList::empty(py);
    for e in a.elements() { data.append(element(py, &e)?)?; }
    result.set_item("shape", a.shape())?;
    result.set_item("data", data)?;
    result.set_item("prototype", element(py, &a.prototype())?)?;
    if !a.axis_names().is_empty() { result.set_item("axis_names", a.axis_names().iter().map(|n| n.as_deref()).collect::<Vec<_>>())?; }
    if a.has_keys() {
        result.set_item(
            "axis_keys",
            a.axis_keys().iter().map(|k| k.as_ref().map(|k| k.names().iter().map(|k| k.to_string()).collect::<Vec<_>>())).collect::<Vec<_>>(),
        )?;
    }
    Ok(result.unbind())
}

#[pyclass(frozen, name = "_Array")]
struct PyArray { inner: Value }

#[pymethods]
impl PyArray {
    #[new]
    fn new(raw: &Bound<'_, PyDict>) -> PyResult<Self> { Ok(Self { inner: import_array(raw, 0)? }) }
    #[getter]
    fn shape(&self) -> Vec<usize> { self.inner.shape().to_vec() }
    #[getter]
    fn axis_names(&self) -> Vec<Option<String>> { (0..self.inner.shape().len()).map(|a| self.inner.axis_name(a).map(|n| n.to_string())).collect() }
    fn with_axis_names(&self, names: Vec<Option<String>>) -> PyResult<Self> {
        if names.len() != self.inner.shape().len() { return Err(PyValueError::new_err("axis_names must have one entry per axis")); }
        let inner =
            self.inner.clone().with_axis_names(names.into_iter().map(|n| n.map(Into::into)).collect()).map_err(|k| PyValueError::new_err(k.to_string()))?;
        Ok(Self { inner })
    }
    #[getter]
    fn axis_keys(&self) -> Vec<Option<Vec<String>>> {
        (0..self.inner.shape().len()).map(|a| self.inner.keys(a).map(|k| k.names().iter().map(|s| s.to_string()).collect())).collect()
    }
    fn with_axis_keys(&self, keys: Vec<Option<Vec<String>>>) -> PyResult<Self> {
        if keys.len() != self.inner.shape().len() { return Err(PyValueError::new_err("axis_keys must have one entry per axis")); }
        let keys = keys
            .into_iter()
            .map(|k| k.map(|names| crate::keyed::Keys::new(names.into_iter().map(Into::into).collect())).transpose())
            .collect::<Result<_, _>>()
            .map_err(|k| PyValueError::new_err(k.to_string()))?;
        Ok(Self { inner: self.inner.clone().with_keys(keys).map_err(|k| PyValueError::new_err(k.to_string()))? })
    }
    #[getter]
    fn is_atom(&self) -> bool { self.inner.is_atom() }
    #[getter]
    fn needs_session(&self) -> PyResult<bool> { self.inner.export_context().map_err(|k| PyValueError::new_err(k.to_string())) }
    fn parts(&self, py: Python<'_>) -> PyResult<Py<PyDict>> { array(py, &self.inner) }
    fn __repr__(&self) -> String { self.inner.to_string() }
    fn scalar(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        if !self.inner.is_singleton() { return Err(PyValueError::new_err("conversion requires a singleton array")); }
        if matches!(self.inner.at(0), Value::Array(_)) { return Err(PyTypeError::new_err("conversion requires a simple scalar")); }
        array(py, &self.inner.at(0))
    }
    fn cells(&self) -> PyResult<Vec<Self>> {
        let rank = self.inner.shape().len().checked_sub(1).ok_or_else(|| PyTypeError::new_err("a scalar has no major cells"))?;
        self.inner
            .cells(rank)
            .and_then(|c| c.collect())
            .map(|a| a.into_iter().map(|inner| Self { inner }).collect())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }
    fn select(&self, py: Python<'_>, parts: Vec<Option<PyRef<'_, PyArray>>>) -> PyResult<Py<PyDict>> {
        let span = Span { source: Source::new("<index>", "[]"), range: 0..2 };
        let execution = crate::execution::Execution::default();
        let parts = parts.into_iter().map(|a| a.map(|a| a.inner.clone())).collect::<Vec<_>>();
        let mut result = Evaluation::default();
        match crate::primitive::select(&self.inner, &parts, &execution.at(&span)) {
            Ok(Value::Function(f)) => result.function = Some(f),
            Ok(a) => result.value = Some(a),
            Err(e) => result.error = Some(e),
        }
        response(py, result)
    }
}

#[pyclass(frozen, name = "_Function")]
struct PyFunction { inner: Function }

fn operand(value: &Bound<'_, PyAny>) -> PyResult<Operand> {
    if let Ok(f) = value.extract::<PyRef<'_, PyFunction>>() { return Ok(Operand::Function(f.inner.clone())); }
    Ok(Operand::Value(value.extract::<PyRef<'_, PyArray>>()?.inner.clone()))
}

#[pymethods]
impl PyFunction {
    #[staticmethod]
    fn builtin(name: &str) -> PyResult<Self> {
        Function::builtin(name).map(|inner| Self { inner }).ok_or_else(|| PyValueError::new_err("unknown builtin function"))
    }
    #[staticmethod]
    fn late_bound(py: Python<'_>, expression: &str) -> PyResult<Py<PyDict>> {
        let result = match Function::late_bound(expression) {
            Ok(function) => Evaluation { function: Some(function), ..Evaluation::default() },
            Err(error) => Evaluation { error: Some(error), ..Evaluation::default() },
        };
        response(py, result)
    }
    #[staticmethod]
    fn build(kind: &str, values: Vec<Bound<'_, PyAny>>) -> PyResult<Self> {
        let operands = values.iter().map(operand).collect::<PyResult<Vec<_>>>()?;
        Function::build(kind, operands).map(|inner| Self { inner }).map_err(|e| PyValueError::new_err(e.to_string()))
    }
    #[getter]
    fn needs_session(&self) -> PyResult<bool> { self.inner.export_context().map_err(|e| PyValueError::new_err(e.to_string())) }
    fn __repr__(&self) -> String { self.inner.apl() }
}
struct Request {
    code: Option<String>,
    function: Option<Function>,
    args: Vec<Value>,
    bindings: Vec<(String, Operand)>,
    options: EvalOptions,
    reply: mpsc::Sender<Result<Evaluation, &'static str>>,
}
impl Request {
    fn run(&mut self, session: &mut Session) -> Result<Evaluation, &'static str> {
        for (name, value) in self.bindings.drain(..) {
            match value { Operand::Value(a) => session.set(&name, a), Operand::Function(f) => session.set_function(&name, f), _ => unreachable!() }
            .map_err(|_| "binding requires an ordinary APL name and an exportable value")?;
        }
        let options = std::mem::take(&mut self.options);
        Ok(match (&self.code, &self.function) {
            (Some(code), _) => session.eval_with(code, options),
            (_, Some(function)) => session.call_function_with(function, &self.args, options),
            _ => Evaluation::default(),
        })
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
        crate::execution::thread()
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
        function: Option<PyRef<'_, PyFunction>>,
        args: Vec<PyRef<'_, PyArray>>,
        bindings: Vec<(String, Bound<'_, PyAny>)>,
        timeout: Option<f64>,
        echo: bool,
    ) -> PyResult<Py<PyDict>> {
        if code.is_some() && function.is_some() { return Err(PyValueError::new_err("choose code or function, not both")); }
        let options = options(timeout, echo)?;
        let interrupt = options.interrupt.clone();
        let (reply, mut receiver) = mpsc::channel();
        let request = Request {
            code,
            function: function.map(|f| f.inner.clone()),
            args: args.iter().map(|a| a.inner.clone()).collect(),
            bindings: bindings.into_iter().map(|(n, a)| Ok((n, operand(&a)?))).collect::<PyResult<_>>()?,
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

fn response(py: Python<'_>, result: Evaluation) -> PyResult<Py<PyDict>> {
    fn location(py: Python<'_>, span: &Span) -> PyResult<Py<PyDict>> {
        let source = PyDict::new(py);
        source.set_item("name", &span.source.name)?;
        source.set_item("text", &span.source.text)?;
        let d = PyDict::new(py);
        d.set_item("source", source)?;
        d.set_item("span", (span.range.start, span.range.end))?;
        Ok(d.unbind())
    }
    let value = if let Some(inner) = result.value { Some(Py::new(py, PyArray { inner })?.into_any()) } else if let Some(inner) = result.function { Some(Py::new(py, PyFunction { inner })?.into_any()) } else { None };
    let error = result
        .error
        .as_ref()
        .map(|e| -> PyResult<Py<PyDict>> {
            let d = location(py, &e.span)?.into_bound(py);
            d.set_item("kind", e.kind.to_string())?;
            d.set_item("message", &e.message)?;
            d.set_item("display", e.to_string())?;
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
    Ok(crate::with_stack(|| crate::reference::check(&case, crate::EvalOptions { timeout: Some(timeout), ..crate::EvalOptions::default() })).to_string())
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySession>()?;
    m.add_class::<PyArray>()?;
    m.add_class::<PyFunction>()?;
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    m.add_function(wrap_pyfunction!(_check_reference, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    let symbols: Vec<_> = crate::symbols::SYMBOLS.iter().map(|&(g, n, m, d, a)| (g, n, m, d, a, crate::symbols::chord(g))).collect();
    m.add("symbols", symbols)?;
    Ok(())
}
