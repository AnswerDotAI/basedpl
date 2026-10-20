use basedpl::{buffer::{Buffer, DType, FloatWidth, IntBuf, View}, EvalOptions, Evaluation, Function, InterruptHandle, Number, Operand, Session, Value};
use num_bigint::BigInt;
use num_rational::BigRational;
use pyo3::{
    exceptions::{PyBufferError, PyTypeError, PyValueError}, ffi, prelude::*, sync::MutexExt,
    types::{PyBool, PyBoolMethods, PyComplex, PyComplexMethods, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple}, IntoPyObjectExt
};
use rustymath::structure::IndexPart;
use std::{cell::RefCell, collections::HashMap, ffi::c_int, sync::{atomic::{AtomicPtr, Ordering}, Arc, Mutex}};

/// The value that Python's `_array` describes in `raw`, with its floats at `width`. A complex number follows the current settings' kinds.
fn import_array(raw: &Bound<'_, PyDict>, width: FloatWidth) -> PyResult<Value> {
    let element = |o: Bound<'_, PyAny>| -> PyResult<Value> {
        if let Ok(a) = o.extract::<PyRef<'_, PyArray>>() { return a.value().cloned(); }
        if let Ok(f) = o.extract::<PyRef<'_, PyFunction>>() { return Ok(Value::Function(f.inner.clone())); }
        if let Ok(op) = o.extract::<PyRef<'_, PyOperator>>() { return Ok(Value::Operator(op.inner.clone())); }
        if let Ok(d) = o.cast::<PyDict>() { return import_array(d, width); }
        if let Ok(s) = o.cast::<PyString>() {
            return basedpl::protocol::character(s.to_str()?).map(Value::Character).ok_or_else(|| PyValueError::new_err("expected one character"));
        }
        let number = if let Ok(b) = o.cast::<PyBool>() { Ok(Number::from_bool(b.is_true())) }
        else if o.is_instance_of::<PyFloat>() { Ok(Number::float(o.extract::<f64>()?, width)) }
        else if let Ok(z) = o.cast::<PyComplex>() {
            Ok(Number::complex_in_session(num_complex::Complex64::new(z.real(), z.imag()), width))
        } else if o.is_instance_of::<PyInt>() { Ok(o.extract::<BigInt>()?.into()) }
        else if o.is_instance_of::<PyTuple>() { let (n, d) = o.extract::<(BigInt, BigInt)>()?; Number::try_from(BigRational::new_raw(n, d)) }
        else { return Err(PyTypeError::new_err("unsupported BPL element")); };
        number.map(Value::Number).map_err(|k| PyValueError::new_err(k.to_string()))
    };
    let field = |key| raw.get_item(key)?.ok_or_else(|| PyValueError::new_err("missing array field"));
    if let Some(atom) = raw.get_item("atom")? { return element(atom); }
    let shape = field("shape")?.extract::<Vec<usize>>()?;
    let data = field("data")?.try_iter()?.map(|o| element(o?)).collect::<PyResult<Vec<_>>>()?;
    let prototype = element(field("prototype")?)?;
    // Python data takes the import rules. A dict keeps each value's kind, as a JSON object does. Other data becomes floats only when every number converts exactly.
    let record = raw.get_item("axis_keys")?.is_some();
    let mut result = if record { Value::mixed(shape, data, prototype) } else { Value::imported(shape, data, prototype) }
        .map_err(|k| PyValueError::new_err(k.to_string()))?;
    if let Some(names) = raw.get_item("axis_names")? { result = basedpl::keyed::with_names(result, names.extract()?).map_err(PyValueError::new_err)?; }
    let Some(keys) = raw.get_item("axis_keys")? else { return Ok(result); };
    basedpl::keyed::with_key_lists(result, keys.extract()?).map_err(PyValueError::new_err)
}

/// `a`'s parts for Python: its `_Array` for compact items, which Python reads through `tolist` or the buffer protocol, or else a dict of its shape, items, prototype, keys and names.
fn array(py: Python<'_>, a: &Value) -> PyResult<Py<PyAny>> {
    fn element(py: Python<'_>, e: &Value) -> PyResult<Py<PyAny>> {
        Ok(match e {
            Value::Number(n) => {
                if let Some(b) = n.as_bool() { PyBool::new(py, b).to_owned().into_any().unbind() }
                else if let Some(n) = n.as_integer() { n.into_pyobject(py)?.into_any().unbind() }
                else if let Some(n) = n.as_exact() {
                    if n.is_integer() { n.numer().into_pyobject(py)?.into_any().unbind() }
                    else { (n.numer(), n.denom()).into_pyobject(py)?.into_any().unbind() }
                } else if let Some(n) = n.as_complex() { PyComplex::from_doubles(py, n.re, n.im).into_any().unbind() }
                else { PyFloat::new(py, n.as_float().unwrap()).into_any().unbind() }
            }
            Value::Character(c) => PyString::new(py, &c.to_string()).into_any().unbind(),
            a @ Value::Array(_) => array(py, a)?,
            Value::Function(f) => Py::new(py, PyFunction { inner: f.clone() })?.into_any(),
            Value::Operator(op) => Py::new(py, PyOperator { inner: op.clone() })?.into_any(),
            Value::Carried(_) => return Err(PyValueError::new_err("a value that a running ∂ carries has no Python value")),
        })
    }
    if !a.is_atom() && a.view().is_some() { return Ok(Py::new(py, PyArray::from(a.clone()))?.into_any()); }
    let result = PyDict::new(py);
    if a.is_atom() { result.set_item("atom", element(py, a)?)?; return Ok(result.into_any().unbind()); }
    let data = PyList::empty(py);
    for e in a.elements() { data.append(element(py, &e)?)?; }
    result.set_item("shape", a.shape())?;
    result.set_item("data", data)?;
    result.set_item("prototype", element(py, &a.prototype())?)?;
    if !a.axis_names().is_empty() { result.set_item("axis_names", a.axis_names().iter().map(|n| n.as_deref()).collect::<Vec<_>>())?; }
    if a.has_keys() { result.set_item("axis_keys", basedpl::keyed::key_lists(a))?; }
    Ok(result.into_any().unbind())
}

/// A BPL value, or nothing once `own` has moved its items into NumPy.
#[pyclass(name = "_Array")]
struct PyArray { inner: Option<Value> }

impl From<Value> for PyArray {
    fn from(inner: Value) -> Self { Self { inner: Some(inner) } }
}

impl PyArray {
    /// The value, or an error once `own` has moved its items.
    fn value(&self) -> PyResult<&Value> { self.inner.as_ref().ok_or_else(|| PyValueError::new_err("own() moved this array's items into NumPy")) }
}

enum IndexSelection { Planned(rustymath::structure::Plan, bool), Rich(Vec<IndexPart<Value>>) }

fn index_error(error: basedpl::Error) -> PyErr {
    match &error.kind { basedpl::ErrorKind::Array(e) => rustymath_buffer::py_err(e.clone()), _ => PyValueError::new_err(error.to_string()) }
}

fn index_names(value: &Value) -> Option<Vec<String>> {
    value.elements().map(|v| match v.view() {
        Some(View::Characters(chars)) if v.shape().len() <= 1 => Some(chars.iter().collect()), _ => None,
    }).collect()
}

fn selection(value: &Value, index: &Bound<'_, PyAny>, convert: &Bound<'_, PyAny>) -> PyResult<IndexSelection> {
    let cache = RefCell::new(HashMap::<usize, Value>::new());
    let converted = |obj: &Bound<'_, PyAny>| -> PyResult<Value> {
        let id = obj.as_ptr() as usize;
        if let Some(value) = cache.borrow().get(&id) { return Ok(value.clone()); }
        let array = convert.call1((obj,))?;
        let value = array.extract::<PyRef<'_, PyArray>>()?.value()?.clone();
        cache.borrow_mut().insert(id, value.clone());
        Ok(value)
    };
    let parts = rustymath_buffer::selection::parts(index)?;
    let mut rich = false;
    for part in &parts {
        let IndexPart::Axis(part) = part else { continue };
        if !part.is_instance_of::<pyo3::types::PySlice>() && part.extract::<Vec<String>>().is_err() {
            let part = converted(part)?;
            rich |= part.view().is_none() && index_names(&part).is_none();
        }
    }
    if rich {
        // Only masks and bounded slices need Python policy bound to a source axis. Other rich values stay untouched for BPL's coordinate/path lowering.
        let bound = parts.iter().any(|p| p.axis().is_some_and(|p| p.is_instance_of::<pyo3::types::PySlice>() || converted(p).is_ok_and(|v| matches!(v.view(), Some(View::Booleans(_))))));
        let mut axes = vec![0; parts.len()];
        if bound {
            let claims = parts.iter().map(|p| p.as_ref().try_map(|p| -> PyResult<_> {
                if p.is_instance_of::<pyo3::types::PySlice>() { return Ok((None, 1)); }
                let v = converted(p)?;
                Ok((rustymath::structure::sole_name(v.core_layout()).map(str::to_owned), v.index_width()))
            })).collect::<PyResult<Vec<_>>>()?;
            let borrowed = claims.iter().map(|p| p.as_ref().map(|(name, width)| (name.as_deref(), *width))).collect::<Vec<_>>();
            for (axis, part) in rustymath::structure::addressed_parts(value.core_layout(), &borrowed, None).map_err(rustymath_buffer::py_err)? {
                if let IndexPart::Axis(i) = part { axes[i] = axis; }
            }
        }
        let values = parts.iter().enumerate().map(|(i, part)| part.as_ref().try_map(|part| -> PyResult<Value> {
            let axis = axes[i];
            if part.is_instance_of::<pyo3::types::PySlice>() {
                let (selector, _) = rustymath_buffer::selection::selector(value.core_layout(), axis, part, || unreachable!("a slice is decoded before an array"))?;
                let rustymath::structure::Selector::Range { start, len, step } = selector else { unreachable!("a slice gives a range") };
                return Value::integers([len], (0..len).map(|i| start as i64 + i as i64 * step as i64).collect()).map_err(|k| PyValueError::new_err(k.to_string()));
            }
            let indices = converted(part)?;
            if let Some(view @ View::Booleans(_)) = indices.view() {
                let (selector, _) = rustymath_buffer::selection::array_selector(value.core_layout(), axis, indices.core_layout().clone(), view, false)?;
                let rustymath::structure::Selector::Indices { positions, .. } = selector else { unreachable!("a Boolean mask gives positions") };
                return Value::integers([positions.len()], positions.into_iter().map(|p| p as i64).collect()).map_err(|k| PyValueError::new_err(k.to_string()));
            }
            Ok(indices)
        })).collect::<PyResult<Vec<_>>>()?;
        return Ok(IndexSelection::Rich(values));
    }
    rustymath_buffer::selection::select_parts(value.core_layout(), parts,
        |obj| {
            if obj.is_instance_of::<pyo3::types::PySlice>() { return Ok(None); }
            Ok(rustymath::structure::sole_name(converted(obj)?.core_layout()).map(str::to_owned))
        },
        |layout, axis, obj| {
            let indices = converted(obj)?;
            if indices.view().is_none() && let Some(names) = index_names(&indices) {
                let positions = names.iter().map(|n| rustymath_buffer::selection::key(layout, axis, n)).collect::<PyResult<Vec<_>>>()?;
                return Ok((rustymath::structure::Selector::Indices { layout: indices.core_layout().clone(), positions }, false));
            }
            let items = indices.view().ok_or_else(|| PyTypeError::new_err("a Python index array must contain ordinary numbers or keys"))?;
            rustymath_buffer::selection::array_selector(layout, axis, indices.core_layout().clone(), items, indices.is_atom())
        },
    ).map(|(plan, atom)| IndexSelection::Planned(plan, atom))
}

/// A BPL value lent read-only to one buffer export, with the shape and strides of the export. The export holds this object, which keeps all three valid.
#[pyclass(frozen)]
struct Lent { value: Value, shape: Box<[isize]>, strides: Box<[isize]> }

/// The array whose compact items an export lends for `value`, with the offset of `value`'s first item in them and, for a structural copy read through an affine map, each axis's stride in items. A copy that hasn't been built lends its source's items this way. Otherwise `value` lends its own items where they're compact. `None` where neither applies.
fn lendable(value: &Value) -> Option<(Value, usize, Option<Vec<isize>>)> {
    if let Some((source, offset, strides)) = value.affine().filter(|(source, ..)| source.view().is_some()) { return Some((source, offset, Some(strides))); }
    value.view().map(|_| (value.clone(), 0, None))
}

/// Items that `own` moved out of BPL, which NumPy views writably through `data`, their address, as `dtype`. NumPy can write any bytes into them, so Booleans are held as `u8` and characters as `i32`, which have no invalid values.
#[pyclass(frozen)]
struct Owned { dtype: DType, items: Buffer, data: AtomicPtr<u8>, shape: Box<[isize]>, strides: Box<[isize]> }

impl Owned {
    fn new(items: Buffer, shape: &[usize]) -> Self {
        let dtype = items.dtype();
        // SAFETY: `bool` and `u8`, and `char` and `i32`, have the same size and alignment, and every `bool` is a valid `u8` and every `char` a valid `i32`.
        let mut items = match items {
            Buffer::Booleans(v) => Buffer::Integers(IntBuf::U8(unsafe { retyped(v) })),
            Buffer::Characters(v) => Buffer::Integers(IntBuf::I32(unsafe { retyped(v) })),
            items => items,
        };
        let data = AtomicPtr::new(items.as_mut_ptr());
        let (shape, strides) = rustymath_buffer::geometry(dtype, shape);
        Self { dtype, items, data, shape, strides }
    }
}

/// `items` as a `Vec<U>` over the same allocation, with no copy.
///
/// # Safety
///
/// `T` and `U` must have the same size and alignment, and every `T` must be a valid `U`.
unsafe fn retyped<T, U>(items: Vec<T>) -> Vec<U> {
    let mut items = std::mem::ManuallyDrop::new(items);
    // SAFETY: the caller guarantees that the layouts match and that each item is a valid `U`.
    unsafe { Vec::from_raw_parts(items.as_mut_ptr().cast(), items.len(), items.capacity()) }
}

#[pymethods]
impl Owned {
    unsafe fn __getbuffer__(slf: Bound<'_, Self>, view: *mut ffi::Py_buffer, flags: c_int) -> PyResult<()> {
        let this = slf.get();
        // SAFETY: `data` addresses `items`, which the object owns with the shape and strides and never changes, and the export holds the object.
        unsafe { rustymath_buffer::export(view, flags, slf.as_any(), this.dtype, this.data.load(Ordering::Relaxed), this.items.len(), &this.shape, &this.strides, false) }
    }
}

#[pymethods]
impl PyArray {
    /// Reads a compact buffer or sibling-array snapshot with its dtype and labels.
    #[staticmethod]
    fn numeric(data: &Bound<'_, PyAny>) -> PyResult<Option<Self>> {
        let Some((read, layout)) = rustymath_buffer::read_array(data)? else { return Ok(None) };
        let value = Value::imported_buffer(layout.shape(), read.into_buffer()).and_then(|v| v.with_core_layout(layout));
        value.map(|v| Some(v.into())).map_err(|k| PyValueError::new_err(k.to_string()))
    }
    fn copy(&self) -> PyResult<Self> { Ok(self.value()?.clone().into()) }
    #[getter]
    fn shape(&self) -> PyResult<Vec<usize>> { Ok(self.value()?.shape().to_vec()) }
    #[getter]
    fn names(&self) -> PyResult<Vec<Option<String>>> {
        let value = self.value()?;
        Ok((0..value.shape().len()).map(|a| value.axis_name(a).map(|n| n.to_string())).collect())
    }
    fn with_names(&self, names: Option<Vec<Option<String>>>) -> PyResult<Self> {
        let value = self.value()?;
        let layout = value.core_layout().clone().with_names(names.unwrap_or_default().into_iter().map(|n| n.map(Arc::from)).collect()).map_err(rustymath_buffer::py_err)?;
        Ok(value.clone().with_core_layout(layout).map_err(|k| PyValueError::new_err(k.to_string()))?.into())
    }
    #[getter]
    fn keys(&self) -> PyResult<Vec<Option<Vec<Option<&str>>>>> { Ok(basedpl::keyed::key_lists(self.value()?)) }
    fn with_keys(&self, keys: Option<Vec<Option<Vec<Option<String>>>>>) -> PyResult<Self> {
        let value = self.value()?;
        let keys = rustymath_buffer::axis_keys(keys.unwrap_or_default())?;
        let layout = value.core_layout().clone().with_keys(keys).map_err(rustymath_buffer::py_err)?;
        Ok(value.clone().with_core_layout(layout).map_err(|k| PyValueError::new_err(k.to_string()))?.into())
    }
    fn __rustymath_array__(&self, py: Python<'_>) -> PyResult<(Self, Py<PyAny>, Vec<Option<String>>)> {
        if !self.exports()? { return Err(PyTypeError::new_err("only ordinary compact values can cross to Rustymath; convert exact or mixed values explicitly")); }
        Ok((self.copy()?, self.keys()?.into_py_any(py)?, self.names()?))
    }
    #[getter]
    fn is_atom(&self) -> PyResult<bool> { Ok(self.value()?.is_atom()) }
    #[getter]
    fn is_integral(&self) -> PyResult<bool> { Ok(self.value()?.is_integral()) }
    fn parts(&self, py: Python<'_>) -> PyResult<Py<PyAny>> { array(py, self.value()?) }
    /// The items as nested lists of Python scalars, or one scalar at rank 0, read without NumPy. Only compact items convert.
    fn tolist<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let value = self.value()?;
        let items = value.view().ok_or_else(|| PyTypeError::new_err("only compact items convert to a list"))?;
        rustymath_buffer::tolist(py, items, value.shape())
    }
    /// The string that an unkeyed, unnamed character vector holds, or `None`.
    #[getter]
    fn text(&self) -> PyResult<Option<String>> {
        let value = self.value()?;
        let plain = value.shape().len() == 1 && !value.has_keys() && value.axis_names().is_empty();
        Ok(match value.view() { Some(View::Characters(c)) if plain => Some(c.iter().collect()), _ => None })
    }
    /// Whether NumPy can read the items through the buffer protocol: compact storage, or a structural copy that reads compact items through an affine map.
    #[getter]
    fn exports(&self) -> PyResult<bool> { Ok(lendable(self.value()?).is_some()) }
    /// Exports the items read-only and without a copy, so that NumPy can't change a BPL value through them. An unbuilt structural copy with an affine read, such as windows or a reversal, lends its source's items with strides, without being built.
    unsafe fn __getbuffer__(slf: Bound<'_, Self>, view: *mut ffi::Py_buffer, flags: c_int) -> PyResult<()> {
        let value = slf.borrow().value()?.clone();
        let (items, offset, strides) = lendable(&value).ok_or_else(|| PyBufferError::new_err("an array of mixed or exact items has no buffer"))?;
        let dtype = items.view().expect("lendable items are compact").dtype();
        let (shape, strides) = match strides { Some(s) => rustymath_buffer::strided_geometry(dtype, value.shape(), &s), None => rustymath_buffer::geometry(dtype, value.shape()) };
        // An atom holds its items inline, so the pointer comes from the copy that `lent` keeps.
        let lent = Bound::new(slf.py(), Lent { value: items, shape, strides })?;
        let this = lent.get();
        // SAFETY: the offset and strides reach only items of `this.value`, as the copy's map does.
        let data = unsafe { this.value.view().expect("the items are compact").as_ptr().add(offset * dtype.bytes()) };
        // SAFETY: `lent` never changes its value, shape or strides, and the export holds `lent`.
        unsafe { rustymath_buffer::export(view, flags, lent.as_any(), dtype, data.cast_mut(), value.shape().iter().product(), &this.shape, &this.strides, true) }
    }
    /// The items for NumPy to view writably: moved with no copy where nothing else holds them, and copied otherwise. For items it can't move, such as exact numbers, it still empties the array and returns `None`, so the caller copies them first.
    fn own(&mut self) -> PyResult<Option<Owned>> {
        self.value()?;
        let value = self.inner.take().expect("checked above");
        let shape = value.shape().to_vec();
        Ok(value.into_buffer().ok().map(|items| Owned::new(items, &shape)))
    }
    fn __repr__(&self) -> PyResult<String> { Ok(self.value()?.to_string()) }
    fn literal(&self) -> PyResult<String> { Ok(self.value()?.literal()) }
    fn native_atom(&self) -> PyResult<Self> {
        let value = self.value()?;
        if !value.is_singleton() { return Err(PyValueError::new_err("conversion requires a singleton array")); }
        let item = value.at(0);
        if matches!(item, Value::Array(_)) { return Err(PyTypeError::new_err("conversion requires an atom")); }
        Ok(item.into())
    }
    fn atom(&self, py: Python<'_>) -> PyResult<Py<PyAny>> { array(py, self.native_atom()?.value()?) }
    fn cells(&self) -> PyResult<Vec<Self>> {
        let value = self.value()?;
        if value.shape().is_empty() { return Err(PyTypeError::new_err("a unit has no major cells")); }
        let cells = value.major_cells().map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(cells.into_iter().map(Self::from).collect())
    }
    fn select(&self, py: Python<'_>, index: &Bound<'_, PyAny>, convert: &Bound<'_, PyAny>) -> PyResult<Py<PyDict>> {
        let value = match selection(self.value()?, index, convert)? {
            IndexSelection::Planned(plan, atom) => self.value()?.selected(plan, atom),
            IndexSelection::Rich(parts) => self.value()?.select(&parts),
        }.map_err(index_error)?;
        let mut result = Evaluation::default();
        match value { Value::Function(f) => result.function = Some(f), a => result.value = Some(a) }
        response(py, result)
    }
    fn assign(&self, index: &Bound<'_, PyAny>, values: PyRef<'_, PyArray>, convert: &Bound<'_, PyAny>) -> PyResult<Self> {
        match selection(self.value()?, index, convert)? {
            IndexSelection::Planned(plan, atom) => self.value()?.assigned(plan, atom, values.value()?),
            IndexSelection::Rich(parts) => self.value()?.assigned_parts(&parts, values.value()?),
        }.map(Self::from).map_err(index_error)
    }
    fn contains(&self, py: Python<'_>, query: PyRef<'_, PyArray>, session: PyRef<'_, PySession>) -> PyResult<bool> {
        let (value, query) = (self.value()?.clone(), query.value()?.clone());
        let mut guard = session.session.lock_py_attached(py).unwrap();
        let session = &mut *guard;
        py.detach(|| value.contains(&query, session)).map_err(|e| match &e.kind {
            basedpl::ErrorKind::Array(e) if e.kind == rustymath::error::Kind::Rank => PyTypeError::new_err(e.message.to_string()),
            _ => PyValueError::new_err(e.to_string()),
        })
    }
}

#[pyclass(frozen, name = "_Function")]
struct PyFunction { inner: Function }

fn operand(value: &Bound<'_, PyAny>) -> PyResult<Operand> {
    if let Ok(f) = value.extract::<PyRef<'_, PyFunction>>() { return Ok(Operand::Function(f.inner.clone())); }
    Ok(Operand::Value(value.extract::<PyRef<'_, PyArray>>()?.value()?.clone()))
}

#[pymethods]
impl PyFunction {
    #[staticmethod]
    fn builtin(name: &str) -> PyResult<Self> {
        Function::builtin(name).map(|inner| Self { inner }).ok_or_else(|| PyValueError::new_err("unknown builtin function"))
    }
    #[staticmethod]
    fn build(kind: &str, values: Vec<Bound<'_, PyAny>>) -> PyResult<Self> {
        let operands = values.iter().map(operand).collect::<PyResult<Vec<_>>>()?;
        Function::build(kind, operands).map(|inner| Self { inner }).map_err(|e| PyValueError::new_err(e.to_string()))
    }
    fn __repr__(&self) -> String { self.inner.bpl() }
    /// This function with `g` attached as its inverse, as BPL's `f⁻¹←g` attaches it.
    fn with_inverse(&self, g: PyRef<'_, PyFunction>) -> Self { Self { inner: self.inner.with_inverse(g.inner.clone()) } }
    /// This function with `g` attached as its derivative, as BPL's `f∂←g` attaches it.
    fn with_derivative(&self, g: PyRef<'_, PyFunction>) -> Self { Self { inner: self.inner.with_derivative(g.inner.clone()) } }
    fn parts(&self, py: Python<'_>) -> PyResult<Option<(String, Vec<Py<PyAny>>)>> {
        let Some((kind, operands)) = self.inner.parts() else { return Ok(None); };
        let args = operands.into_iter().map(|a| match a {
            Operand::Function(inner) => Py::new(py, PyFunction { inner }).map(|f| f.into_any()),
            Operand::Value(inner) => Py::new(py, PyArray::from(inner)).map(|a| a.into_any()),
        }).collect::<PyResult<Vec<_>>>()?;
        Ok(Some((kind, args)))
    }
}

/// An operator held as a value, as a module record holds it.
#[pyclass(frozen, name = "_Operator")]
struct PyOperator { inner: basedpl::Operator }

#[pymethods]
impl PyOperator {
    /// The function this operator derives from its operand, or from both operands of a dyadic operator.
    #[pyo3(signature = (left, right=None))]
    fn derive(&self, left: &Bound<'_, PyAny>, right: Option<&Bound<'_, PyAny>>) -> PyResult<PyFunction> {
        let right = right.map(operand).transpose()?;
        self.inner.derive(operand(left)?, right).map(|inner| PyFunction { inner }).map_err(|e| PyValueError::new_err(e.to_string()))
    }
    fn __repr__(&self) -> String { self.inner.to_string() }
}
/// What a request runs: source code, or a function called with arguments.
enum Run { Code(String), Call(Function, Vec<Value>) }
struct EvalRequest { run: Option<Run>, bindings: Vec<(String, Operand)>, options: EvalOptions }
impl EvalRequest {
    fn run(&mut self, session: &mut Session) -> Result<Evaluation, &'static str> {
        for (name, value) in self.bindings.drain(..) {
            match value {
                Operand::Value(a) => session.set(&name, a), Operand::Function(f) => session.set_function(&name, f)
            }.map_err(|_| "binding requires an ordinary BPL name and an exportable value")?;
        }
        let options = std::mem::take(&mut self.options);
        Ok(match &self.run {
            Some(Run::Code(code)) => session.eval_with(code, options), Some(Run::Call(function, args)) => session.call_function_with(function, args, options),
            None => Evaluation::default()
        })
    }
}

#[pyclass(frozen, name = "_Session")]
struct PySession { session: Mutex<Session>, active: Mutex<Option<InterruptHandle>> }

#[pymethods]
impl PySession {
    #[new]
    fn new() -> Self { Self { session: Mutex::new(Session::new()), active: Mutex::new(None) } }
    /// Runs BPL `code` after binding `bindings`. `show` is called with each output event as BPL produces it, so output and input prompts appear in order.
    #[pyo3(signature = (*, code=None, bindings=Vec::new(), timeout=None, echo=false, show=None))]
    fn eval(
        &self, py: Python<'_>, code: Option<String>, bindings: Vec<(String, Bound<'_, PyAny>)>, timeout: Option<f64>, echo: bool, show: Option<Py<PyAny>>
    ) -> PyResult<Py<PyDict>> {
        let bindings = bindings.into_iter().map(|(n, a)| Ok((n, operand(&a)?))).collect::<PyResult<_>>()?;
        self.request(py, EvalRequest { run: code.map(Run::Code), bindings, options: options(timeout, echo)? }, show)
    }
    /// Calls `function` with `args`, as `eval` runs code. Each argument that `weak` marks came from a Python scalar, and takes the width that NEP 50 gives it beside the others (`Value::weakened`).
    #[pyo3(signature = (function, args, *, weak=Vec::new(), timeout=None, show=None))]
    fn call(
        &self, py: Python<'_>, function: PyRef<'_, PyFunction>, args: Vec<PyRef<'_, PyArray>>, weak: Vec<bool>, timeout: Option<f64>, show: Option<Py<PyAny>>
    ) -> PyResult<Py<PyDict>> {
        let values = args.iter().map(|a| a.value().cloned()).collect::<PyResult<Vec<_>>>()?;
        let is_weak = |i: usize| weak.get(i).copied().unwrap_or(false);
        let strong: Vec<Value> = values.iter().enumerate().filter(|&(i, _)| !is_weak(i)).map(|(_, v)| v.clone()).collect();
        let values = values.into_iter().enumerate().map(|(i, v)| if is_weak(i) { v.weakened(&strong) } else { v }).collect();
        self.request(py, EvalRequest { run: Some(Run::Call(function.inner.clone(), values)), bindings: Vec::new(), options: options(timeout, false)? }, show)
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
        }).transpose()
    }
    /// The function that `expression` writes, late-bound, with its numbers read at the session's default width.
    fn late_bound(&self, py: Python<'_>, expression: &str) -> PyResult<Py<PyDict>> {
        let result = match self.session.lock_py_attached(py).unwrap().late_bound(expression) {
            Ok(function) => Evaluation { function: Some(function), ..Evaluation::default() },
            Err(error) => Evaluation { error: Some(error), ..Evaluation::default() },
        };
        response(py, result)
    }
    /// The array that the Python data in `raw` describes. Python floats take the session's default float width.
    fn array(&self, py: Python<'_>, raw: &Bound<'_, PyDict>) -> PyResult<PyArray> {
        let width = self.session.lock_py_attached(py).unwrap().float_width();
        Ok(import_array(raw, width)?.into())
    }
    /// Gives the session the display defaults of BPL's REPL and Jupyter kernel, for the BPL magics in IPython.
    fn make_interactive(&self, py: Python<'_>) { self.session.lock_py_attached(py).unwrap().make_interactive() }
    #[pyo3(signature = (prefix="", classes=vec![2, 3, 4], complete=false))]
    fn names(&self, py: Python<'_>, prefix: &str, classes: Vec<i64>, complete: bool) -> Vec<String> {
        let session = self.session.lock_py_attached(py).unwrap();
        if complete { session.complete(prefix) } else { session.name_list(&classes, prefix) }
    }
    fn interrupt(&self) { if let Some(active) = &*self.active.lock().unwrap() { active.interrupt(); } }
}

impl PySession {
    /// Runs `request` with Python's Ctrl-C, `input()` and the `show` callback attached, and gives its result as a dict.
    fn request(&self, py: Python<'_>, mut request: EvalRequest, show: Option<Py<PyAny>>) -> PyResult<Py<PyDict>> {
        let signal = Arc::new(Mutex::new(None));
        let shown = Arc::new(Mutex::new(Vec::new()));
        request.options.poll = Some(ctrl_c(signal.clone()));
        request.options.input = Some(Arc::new(PythonInput { caught: signal.clone() }));
        request.options.output = show.map(|show| stream(show, shown.clone(), signal.clone()));
        let mut result = {
            let mut guard = self.session.lock_py_attached(py).unwrap();
            let session = &mut *guard;
            *self.active.lock().unwrap() = Some(request.options.interrupt.clone());
            let result = py.detach(move || request.run(session));
            *self.active.lock().unwrap() = None;
            result.map_err(PyValueError::new_err)?
        };
        result.output.append(&mut shown.lock().unwrap());
        if let Some(e) = signal.lock().unwrap().take() {
            e.value(py).setattr("output", result.output_text())?;
            e.value(py).setattr("events", output(py, &result.output)?)?;
            return Err(e);
        }
        response(py, result)
    }
}

/// A poll that interrupts the evaluation when Python has a pending signal, such as Ctrl-C, or when an output callback raised. It keeps the error to raise.
fn ctrl_c(caught: Arc<Mutex<Option<PyErr>>>) -> basedpl::Poll {
    Arc::new(move || {
        if caught.lock().unwrap().is_some() { return true; }
        let Err(e) = Python::attach(|py| py.check_signals()) else { return false };
        *caught.lock().unwrap() = Some(e);
        true
    })
}

/// An output sink that keeps each event in `shown` and calls `show` with it. An error that `show` raises goes to `caught`, which stops the evaluation through `ctrl_c`.
fn stream(show: Py<PyAny>, shown: Arc<Mutex<Vec<basedpl::Output>>>, caught: Arc<Mutex<Option<PyErr>>>) -> basedpl::OutputSink {
    Arc::new(move |event| {
        shown.lock().unwrap().push(event.clone());
        let called = Python::attach(|py| python(py, &basedpl::protocol::output(event)).and_then(|e| show.call1(py, (e,))).map(drop));
        if let Err(e) = called { caught.lock().unwrap().get_or_insert(e); }
    })
}

/// Python's standard input. Each line comes from `input()`, which shows a notebook's input box under IPython. A Ctrl-C while `input()` waits goes to `caught`, as a signal does in `ctrl_c`.
struct PythonInput { caught: Arc<Mutex<Option<PyErr>>> }
impl basedpl::Input for PythonInput {
    fn line(&self, prompt: &str) -> std::io::Result<Option<String>> {
        Python::attach(|py| match py.import("builtins").and_then(|b| b.call_method1("input", (prompt,))).and_then(|line| line.extract::<String>()) {
            Ok(line) => Ok(Some(line)),
            Err(e) if e.is_instance_of::<pyo3::exceptions::PyEOFError>(py) => Ok(None),
            Err(e) if e.is_instance_of::<pyo3::exceptions::PyKeyboardInterrupt>(py) => {
                *self.caught.lock().unwrap() = Some(e);
                Err(std::io::ErrorKind::Interrupted.into())
            }
            Err(e) => Err(std::io::Error::other(e.to_string())),
        })
    }
    fn rest(&self) -> std::io::Result<Vec<u8>> {
        Python::attach(|py| -> PyResult<String> { py.import("sys")?.getattr("stdin")?.call_method0("read")?.extract() }).map(String::into_bytes)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

fn options(timeout: Option<f64>, echo: bool) -> PyResult<basedpl::EvalOptions> {
    let timeout = timeout
        .map(|seconds| std::time::Duration::try_from_secs_f64(seconds).map_err(|_| PyValueError::new_err("timeout must be finite and nonnegative")))
        .transpose()?;
    Ok(basedpl::EvalOptions { timeout, echo, ..basedpl::EvalOptions::default() })
}

/// A JSON value as the matching Python object.
fn python(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    use serde_json::Value as Json;
    match value {
        Json::Null => Ok(py.None()),
        Json::Bool(b) => b.into_py_any(py),
        Json::Number(n) => n.as_i64().map_or_else(|| n.as_f64().into_py_any(py), |i| i.into_py_any(py)),
        Json::String(s) => s.into_py_any(py),
        Json::Array(items) => items.iter().map(|v| python(py, v)).collect::<PyResult<Vec<_>>>()?.into_py_any(py),
        Json::Object(fields) => {
            let d = PyDict::new(py);
            for (k, v) in fields { d.set_item(k, python(py, v)?)?; }
            d.into_py_any(py)
        }
    }
}

/// A result as the dict that the JSON protocol sends, holding native arrays and functions.
fn response(py: Python<'_>, result: Evaluation) -> PyResult<Py<PyDict>> {
    let value = if let Some(inner) = result.value { Some(Py::new(py, PyArray::from(inner))?.into_any()) }
    else if let Some(inner) = result.function { Some(Py::new(py, PyFunction { inner })?.into_any()) }
    else if let Some(inner) = result.operator { Some(Py::new(py, PyOperator { inner })?.into_any()) }
    else { None };
    let d = PyDict::new(py);
    d.set_item("value", value)?;
    d.set_item("output", output(py, &result.output)?)?;
    d.set_item("error", result.error.as_ref().map(|e| error(py, e)).transpose()?)?;
    Ok(d.unbind())
}

fn output(py: Python<'_>, events: &[basedpl::Output]) -> PyResult<Py<PyAny>> { python(py, &events.iter().map(basedpl::protocol::output).collect()) }

/// BPL source for a value captured from another interpreter, given as JSON.
#[pyfunction]
fn _captured_literal(value: &str) -> PyResult<String> {
    let value = serde_json::from_str(value).map_err(|e| PyValueError::new_err(e.to_string()))?;
    basedpl::reference::expected_array(&value).map(|v| v.literal()).ok_or_else(|| PyValueError::new_err("invalid captured array"))
}

/// The cases of `text`, in the format of a `.bpl` reference file, as `reference::cases` reads them.
#[pyfunction]
fn _reference_cases(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    python(py, &serde_json::Value::Array(basedpl::reference::cases(text).map_err(PyValueError::new_err)?))
}

/// The character offset of each byte offset in `text` that starts a character or ends the text. Python indexes strings by character.
fn char_offsets(text: &str) -> Vec<usize> {
    let mut chars = vec![0; text.len() + 1];
    for (i, (byte, _)) in text.char_indices().enumerate() { chars[byte] = i; }
    chars[text.len()] = text.chars().count();
    chars
}

/// `protocol::error` with character spans. The JSON worker protocol keeps byte spans.
fn error(py: Python<'_>, e: &basedpl::Error) -> PyResult<Py<PyAny>> {
    let span = |s: &basedpl::Span| {
        let chars = char_offsets(&s.source.text);
        serde_json::json!([chars[s.range.start], chars[s.range.end]])
    };
    let mut encoded = basedpl::protocol::error(e);
    encoded["span"] = span(&e.span);
    if let Some(calls) = encoded["calls"].as_array_mut() { for (call, s) in calls.iter_mut().zip(&e.calls) { call["span"] = span(s); } }
    python(py, &encoded)
}

/// The parse tree of `text` as nested `(kind, start, end, children, error)` tuples, with character offsets. `error` is the syntax error of an `Error` node.
#[pyfunction]
fn _parse(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    fn node(py: Python<'_>, n: &basedpl::SyntaxNode, chars: &[usize]) -> PyResult<Py<PyAny>> {
        let children = n.children.iter().map(|c| node(py, c, chars)).collect::<PyResult<Vec<_>>>()?;
        let error = n.error.as_ref().map(|e| error(py, e)).transpose()?;
        (n.kind, chars[n.range.start], chars[n.range.end], children, error).into_py_any(py)
    }
    node(py, &basedpl::syntax_tree(text), &char_offsets(text))
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySession>()?;
    m.add_class::<PyArray>()?;
    m.add_class::<PyFunction>()?;
    m.add_class::<PyOperator>()?;
    m.add_function(wrap_pyfunction!(_captured_literal, m)?)?;
    m.add_function(wrap_pyfunction!(_reference_cases, m)?)?;
    m.add_function(wrap_pyfunction!(_parse, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("symbols", python(m.py(), &basedpl::symbols::rows())?)?;
    m.add("_system_functions", basedpl::system::names().collect::<Vec<_>>())?;
    m.add("_superscripts", basedpl::superscripts())?;
    m.add("_subscripts", basedpl::subscripts())?;
    Ok(())
}
