use crate::{
    array::{generated_len, Layout},
    keyed::Keys,
    ErrorKind, Value,
};

pub(crate) enum Mapping {
    Scalar,
    Linear(usize),
    Positions(Vec<usize>),
    Axes(Vec<(usize, usize, usize)>),
    Keyed(Vec<(usize, usize, Vec<Option<usize>>)>),
}
impl Mapping {
    pub fn index(&self, i: usize) -> usize { self.get(i).expect("positional mapping") }
    pub fn numeric<T: Copy + Default>(&self, data: &[T], i: usize) -> T { self.get(i).map_or_else(T::default, |j| data[j]) }
    pub fn get(&self, i: usize) -> Option<usize> {
        Some(match self {
            Self::Scalar => 0,
            Self::Linear(repeat) => i / repeat,
            Self::Positions(positions) => positions[i],
            Self::Axes(axes) => axes.iter().map(|&(stride, len, source)| i / stride % len * source).sum(),
            Self::Keyed(axes) => axes.iter().try_fold(0, |n, (stride, source, map)| Some(n + map[i / stride % map.len()]? * source))?,
        })
    }
    pub fn contract(left: &Layout, laxis: usize, right: &Layout, raxis: usize) -> Result<Self, ErrorKind> {
        match (left.keys(laxis), right.keys(raxis)) { (Some(x), Some(y)) if x != y => Ok(Self::Positions(y.positions(x, false)?)), _ => Ok(Self::Linear(1)) }
    }
    fn new(input: &Layout, axes: &[usize], result: &Layout, count: usize) -> Self {
        if count == 0 { return Self::Scalar; }
        let (shape, output) = (input.shape(), result.shape());
        let mut strides = vec![1; output.len()];
        for i in (1..output.len()).rev() { strides[i - 1] = strides[i] * output[i]; }
        if axes.iter().enumerate().any(|(a, &b)| input.keys(a).is_some() && input.keys(a) != result.keys(b)) {
            let mut source = 1;
            let mut map = Vec::new();
            for (a, (&len, &b)) in shape.iter().zip(axes).enumerate().rev() {
                let positions = (0..output[b])
                    .map(|i| match (input.keys(a), result.keys(b)) {
                        (Some(src), Some(dst)) => src.position(&dst.names()[i]),
                        _ => Some(if len == 1 { 0 } else { i }),
                    })
                    .collect();
                map.push((strides[b], source, positions));
                source *= len;
            }
            return Self::Keyed(map);
        }
        let size: usize = shape.iter().product();
        if size == 1 { return Self::Scalar; }
        let mut source = 1;
        let mut map = Vec::new();
        for (&len, &axis) in shape.iter().zip(axes).rev() {
            if len != 1 { map.push((strides[axis], len, source)); }
            source *= len;
        }
        let repeat = count / size;
        if map.iter().all(|&(stride, _, source)| stride == source * repeat) { Self::Linear(repeat) } else { Self::Axes(map) }
    }
}

pub(crate) struct Agreement {
    pub layout: Layout,
    pub len: usize,
    pub left: Mapping,
    pub right: Mapping,
}
impl Agreement {
    pub fn new(left: &Layout, right: &Layout) -> Result<Self, ErrorKind> {
        Self::mapped(left, right, &(0..left.shape().len()).collect::<Vec<_>>(), &(0..right.shape().len()).collect::<Vec<_>>())
    }
    pub fn with_axes(left: &Layout, right: &Layout, axes: &[usize]) -> Result<Self, ErrorKind> {
        if left.shape().len() < right.shape().len() { Self::mapped(left, right, axes, &(0..right.shape().len()).collect::<Vec<_>>()) } else { Self::mapped(left, right, &(0..left.shape().len()).collect::<Vec<_>>(), axes) }
    }
    fn mapped(left: &Layout, right: &Layout, xa: &[usize], ya: &[usize]) -> Result<Self, ErrorKind> {
        let mut shape = vec![1; left.shape().len().max(right.shape().len())];
        let mut keys = vec![None; shape.len()];
        for axis in 0..shape.len() {
            let (x, y) = (xa.iter().position(|&a| a == axis), ya.iter().position(|&a| a == axis));
            let (nx, ny) = (x.map_or(1, |a| left.shape()[a]), y.map_or(1, |a| right.shape()[a]));
            let (kx, ky) = (x.and_then(|a| left.keys(a)), y.and_then(|a| right.keys(a)));
            if let (Some(x), Some(y)) = (kx, ky) {
                let key = if x == y { x.clone() } else { Keys::new(x.names().iter().chain(y.names().iter().filter(|k| x.position(k).is_none())).cloned().collect())? };
                shape[axis] = key.names().len();
                keys[axis] = Some(key);
            } else {
                let n = if nx == ny || ny == 1 { nx } else if nx == 1 { ny } else { return Err(ErrorKind::Length); };
                shape[axis] = n;
                keys[axis] = kx.filter(|_| nx == n).or(ky.filter(|_| ny == n)).cloned();
            }
        }
        let len = generated_len(&shape)?;
        let layout = Layout::from(shape).with_keys(keys)?;
        let x = Mapping::new(left, xa, &layout, len);
        let y = Mapping::new(right, ya, &layout, len);
        Ok(Self { layout, len, left: x, right: y })
    }
    pub fn values(&self, left: Option<&Value>, right: &Value, i: usize) -> (Option<Value>, Value) {
        if self.len == 0 {
            let item = |a: &Value| if a.is_empty() { a.prototype() } else { a.at(0) };
            return (left.map(item), item(right));
        }
        let x = left.and_then(|a| self.left.get(i).map(|j| a.at(j)));
        let y = self.right.get(i).map(|j| right.at(j));
        let xf = left.map(|a| x.clone().unwrap_or_else(|| y.as_ref().map_or_else(|| a.prototype(), Value::fill)));
        let yf = y.unwrap_or_else(|| x.as_ref().map_or_else(|| right.prototype(), Value::fill));
        (xf, yf)
    }
}
