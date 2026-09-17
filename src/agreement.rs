use crate::{array::generated_len, ErrorKind};

pub(crate) enum Mapping { Scalar, Linear(usize), Axes(Vec<(usize, usize, usize)>) }
impl Mapping {
    pub fn index(&self, i: usize) -> usize {
        match self {
            Self::Scalar => 0,
            Self::Linear(repeat) => i / repeat,
            Self::Axes(axes) => axes.iter().map(|&(stride, len, source)| i / stride % len * source).sum(),
        }
    }
    fn new(shape: &[usize], axes: &[usize], result: &[usize], count: usize) -> Self {
        if count == 0 { return Self::Scalar; }
        let size: usize = shape.iter().product();
        if size == 1 { return Self::Scalar; }
        let mut strides = vec![1; result.len()];
        for i in (1..result.len()).rev() { strides[i - 1] = strides[i] * result[i]; }
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
    pub shape: Vec<usize>,
    pub len: usize,
    pub left: Mapping,
    pub right: Mapping,
}
impl Agreement {
    pub fn new(left: &[usize], right: &[usize]) -> Result<Self, ErrorKind> {
        Self::mapped(left, right, &(0..left.len()).collect::<Vec<_>>(), &(0..right.len()).collect::<Vec<_>>())
    }
    pub fn with_axes(left: &[usize], right: &[usize], axes: &[usize]) -> Result<Self, ErrorKind> {
        if left.len() < right.len() { Self::mapped(left, right, axes, &(0..right.len()).collect::<Vec<_>>()) } else { Self::mapped(left, right, &(0..left.len()).collect::<Vec<_>>(), axes) }
    }
    fn mapped(left: &[usize], right: &[usize], xa: &[usize], ya: &[usize]) -> Result<Self, ErrorKind> {
        let mut shape = vec![1; left.len().max(right.len())];
        for (input, axes) in [(left, xa), (right, ya)] {
            for (&n, &axis) in input.iter().zip(axes) {
                let d = &mut shape[axis];
                if *d == 1 { *d = n; } else if n != 1 && n != *d { return Err(ErrorKind::Length); }
            }
        }
        let len = generated_len(&shape)?;
        let x = Mapping::new(left, xa, &shape, len);
        let y = Mapping::new(right, ya, &shape, len);
        Ok(Self { shape, len, left: x, right: y })
    }
}
