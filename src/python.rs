use crate::{eval::Operand, EvalOptions, Evaluation, Function, InterruptHandle, Number, Session, Source, Span, Value};
use num_bigint::BigInt;
use num_rational::BigRational;
use pyo3::{
    buffer::PyBuffer,
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    sync::MutexExt,
    types::{PyByteArray, PyComplex, PyComplexMethods, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
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
        .extract::<Vec<Option<Vec<Option<String>>>>>()?
        .into_iter()
        .map(|k| k.map(|names| crate::keyed::Keys::partial(names.into_iter().map(|n| n.map(Into::into)).collect())).transpose())
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
            a.axis_keys().iter().map(|k| k.as_ref().map(|k| k.names().iter().map(|k| k.as_deref().map(str::to_owned)).collect::<Vec<_>>())).collect::<Vec<_>>(),
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
    #[staticmethod]
    fn numeric(shape: Vec<usize>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let py = data.py();
        let inner = match PyBuffer::<i64>::get(data) {
            Ok(b) => Value::integers(shape, b.to_vec(py)?),
            Err(_) => Value::floats(shape, PyBuffer::<f64>::get(data)?.to_vec(py)?),
        };
        inner.map(|inner| Self { inner }).map_err(|k| PyValueError::new_err(k.to_string()))
    }
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
    fn axis_keys(&self) -> Vec<Option<Vec<Option<String>>>> {
        (0..self.inner.shape().len()).map(|a| self.inner.keys(a).map(|k| k.names().iter().map(|s| s.as_deref().map(str::to_owned)).collect())).collect()
    }
    fn with_axis_keys(&self, keys: Vec<Option<Vec<Option<String>>>>) -> PyResult<Self> {
        if keys.len() != self.inner.shape().len() { return Err(PyValueError::new_err("axis_keys must have one entry per axis")); }
        let keys = keys
            .into_iter()
            .map(|k| k.map(|names| crate::keyed::Keys::partial(names.into_iter().map(|n| n.map(Into::into)).collect())).transpose())
            .collect::<Result<_, _>>()
            .map_err(|k| PyValueError::new_err(k.to_string()))?;
        Ok(Self { inner: self.inner.clone().with_keys(keys).map_err(|k| PyValueError::new_err(k.to_string()))? })
    }
    #[getter]
    fn is_atom(&self) -> bool { self.inner.is_atom() }
    fn parts(&self, py: Python<'_>) -> PyResult<Py<PyDict>> { array(py, &self.inner) }
    fn buffer<'py>(&self, py: Python<'py>) -> PyResult<Option<(&'static str, Bound<'py, PyByteArray>)>> {
        fn bytes<'py, const N: usize>(py: Python<'py>, items: impl ExactSizeIterator<Item = [u8; N]>) -> PyResult<Bound<'py, PyByteArray>> {
            PyByteArray::new_with(py, items.len() * N, |buf| {
                for (chunk, item) in buf.chunks_exact_mut(N).zip(items) { chunk.copy_from_slice(&item); }
                Ok(())
            })
        }
        if let Some(v) = self.inner.as_integers() { return Ok(Some(("i8", bytes(py, v.iter().map(|n| n.to_ne_bytes()))?))); }
        let Some(v) = self.inner.as_floats() else { return Ok(None) };
        Ok(Some(("f8", bytes(py, v.iter().map(|n| n.to_ne_bytes()))?)))
    }
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
    fn __repr__(&self) -> String { self.inner.apl() }
    fn parts(&self, py: Python<'_>) -> PyResult<Option<(String, Vec<Py<PyAny>>)>> {
        let Some((kind, operands)) = self.inner.parts() else { return Ok(None); };
        let args = operands
            .into_iter()
            .map(|a| match a {
                Operand::Function(inner) => Py::new(py, PyFunction { inner }).map(|f| f.into_any()),
                Operand::Value(inner) => Py::new(py, PyArray { inner }).map(|a| a.into_any()),
                Operand::Hybrid(_) => unreachable!(),
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(Some((kind, args)))
    }
}
struct EvalRequest {
    code: Option<String>,
    function: Option<Function>,
    args: Vec<Value>,
    bindings: Vec<(String, Operand)>,
    options: EvalOptions,
}
impl EvalRequest {
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

#[pyclass(frozen, name = "_Session")]
struct PySession { session: Mutex<Session>, active: Mutex<Option<InterruptHandle>> }

#[pymethods]
impl PySession {
    #[new]
    fn new() -> Self { Self { session: Mutex::new(Session::new()), active: Mutex::new(None) } }
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
        let signal = Arc::new(Mutex::new(None));
        let mut request = EvalRequest {
            code,
            function: function.map(|f| f.inner.clone()),
            args: args.iter().map(|a| a.inner.clone()).collect(),
            bindings: bindings.into_iter().map(|(n, a)| Ok((n, operand(&a)?))).collect::<PyResult<_>>()?,
            options: EvalOptions { poll: Some(ctrl_c(signal.clone())), ..options(timeout, echo)? },
        };
        let result = {
            let mut guard = self.session.lock_py_attached(py).unwrap();
            let session = &mut *guard;
            *self.active.lock().unwrap() = Some(request.options.interrupt.clone());
            let result = py.detach(move || request.run(session));
            *self.active.lock().unwrap() = None;
            result.map_err(PyValueError::new_err)?
        };
        if let Some(e) = signal.lock().unwrap().take() {
            e.value(py).setattr("output", result.output_text())?;
            e.value(py).setattr("events", output(py, &result.output)?)?;
            return Err(e);
        }
        response(py, result)
    }
    #[pyo3(signature = (*, name=None, function=None))]
    fn inspect(&self, py: Python<'_>, name: Option<String>, function: Option<PyRef<'_, PyFunction>>) -> PyResult<Option<Py<PyDict>>> {
        let info = {
            let session = self.session.lock_py_attached(py).unwrap();
            match (name, function) { (Some(name), _) => session.inspect(&name), (_, Some(f)) => Some(f.inner.inspect(&session)), _ => None }
        };
        info.map(|i| {
            let d = PyDict::new(py);
            d.set_item("kind", i.kind)?;
            d.set_item("source", i.source)?;
            d.set_item("help", i.help)?;
            Ok(d.unbind())
        })
        .transpose()
    }
    #[pyo3(signature = (prefix="", classes=vec![2, 3, 4], complete=false))]
    fn names(&self, py: Python<'_>, prefix: &str, classes: Vec<i64>, complete: bool) -> Vec<String> {
        let session = self.session.lock_py_attached(py).unwrap();
        if complete { session.complete(prefix) } else { session.name_list(&classes, prefix) }
    }
    fn interrupt(&self) { if let Some(active) = &*self.active.lock().unwrap() { active.interrupt(); } }
}

fn ctrl_c(caught: Arc<Mutex<Option<PyErr>>>) -> crate::Poll {
    let last = Mutex::new(Instant::now());
    Arc::new(move || {
        let mut last = last.lock().unwrap();
        if last.elapsed() < Duration::from_millis(10) { return false; }
        *last = Instant::now();
        let Err(e) = Python::attach(|py| py.check_signals()) else { return false };
        *caught.lock().unwrap() = Some(e);
        true
    })
}

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
    d.set_item("output", output(py, &result.output)?)?;
    d.set_item("error", error)?;
    Ok(d.unbind())
}

fn output(py: Python<'_>, events: &[crate::Output]) -> PyResult<Py<PyList>> {
    let events = events
        .iter()
        .map(|e| {
            let d = PyDict::new(py);
            d.set_item("kind", e.kind.name())?;
            d.set_item("data", &e.data)?;
            Ok(d)
        })
        .collect::<PyResult<Vec<_>>>()?;
    Ok(PyList::new(py, events)?.unbind())
}

#[pyfunction]
fn run_cli(args: Vec<String>) -> i32 { crate::cli::run(&args) }

#[pyfunction]
fn _check_reference(case: &str, timeout: f64) -> PyResult<String> {
    let case = serde_json::from_str(case).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let timeout = std::time::Duration::try_from_secs_f64(timeout).map_err(|_| PyValueError::new_err("invalid timeout"))?;
    Ok(crate::reference::check(&case, crate::EvalOptions { timeout: Some(timeout), ..crate::EvalOptions::default() }).to_string())
}

pyo3::create_exception!(_core, JError, pyo3::exceptions::PyException, "A J error, carrying the session's output, including the error display, as its message.");

#[pyclass(frozen, name = "_J")]
struct PyJ { engine: Mutex<crate::j::Engine>, interrupter: crate::j::Interrupter }

#[pymethods]
impl PyJ {
    #[new]
    fn new(lib: std::path::PathBuf) -> PyResult<Self> {
        let engine = crate::j::Engine::new(&lib).map_err(JError::new_err)?;
        Ok(Self { interrupter: engine.interrupter(), engine: Mutex::new(engine) })
    }
    fn run(&self, py: Python<'_>, code: &str) -> PyResult<String> { py.detach(|| self.engine.lock().unwrap().run(code)).map_err(JError::new_err) }
    fn get(&self, name: &str) -> PyResult<(Vec<i64>, crate::j::Data)> { self.engine.lock().unwrap().get(name).map_err(JError::new_err) }
    fn set(&self, name: &str, shape: Vec<i64>, data: crate::j::Data) -> PyResult<()> {
        self.engine.lock().unwrap().set(name, &shape, &data).map_err(JError::new_err)
    }
    fn interrupt(&self) { self.interrupter.interrupt() }
    #[getter]
    fn exited(&self) -> Option<i64> { self.engine.lock().unwrap().exited }
}

#[pyfunction]
fn _run_j_kernel(file: &str, lib: std::path::PathBuf, startup: Option<&str>) -> PyResult<()> {
    crate::j::run_kernel(file, &lib, startup).map_err(|e| JError::new_err(e.to_string()))
}

/// Install a kernelspec for `argv` with message interrupts, under `prefix` or in the user Jupyter directory. Returns its directory.
#[pyfunction]
#[pyo3(signature = (name, argv, display_name, language, prefix=None))]
fn _install_kernelspec(name: &str, argv: Vec<String>, display_name: &str, language: &str, prefix: Option<std::path::PathBuf>) -> PyResult<std::path::PathBuf> {
    let extra = serde_json::Map::from_iter([("interrupt_mode".into(), "message".into())]);
    kernmini::install_kernelspec(name, &argv, display_name, language, extra, prefix.as_deref())
        .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("{e:#}")))
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySession>()?;
    m.add_class::<PyArray>()?;
    m.add_class::<PyFunction>()?;
    m.add_function(wrap_pyfunction!(run_cli, m)?)?;
    m.add_function(wrap_pyfunction!(_check_reference, m)?)?;
    m.add_class::<PyJ>()?;
    m.add("JError", m.py().get_type::<JError>())?;
    m.add_function(wrap_pyfunction!(_run_j_kernel, m)?)?;
    m.add_function(wrap_pyfunction!(_install_kernelspec, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    let symbols: Vec<_> = crate::symbols::SYMBOLS.iter().map(|&(g, n, m, d, a)| (g, n, m, d, a, crate::symbols::chord(g))).collect();
    m.add("symbols", symbols)?;
    m.add("_system_functions", crate::system::names().filter(|name| Function::builtin(name).is_some()).collect::<Vec<_>>())?;
    Ok(())
}
