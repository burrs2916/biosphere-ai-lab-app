use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorShape {
    pub dims: Vec<usize>,
}

impl TensorShape {
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    pub fn scalar() -> Self {
        Self { dims: vec![] }
    }

    pub fn vector(size: usize) -> Self {
        Self { dims: vec![size] }
    }

    pub fn matrix(rows: usize, cols: usize) -> Self {
        Self { dims: vec![rows, cols] }
    }

    pub fn rank(&self) -> usize {
        self.dims.len()
    }

    pub fn size(&self) -> usize {
        self.dims.iter().product()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorSpec {
    pub name: String,
    pub shape: TensorShape,
    pub dtype: TensorDtype,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TensorDtype {
    F32,
    F64,
    I32,
    I64,
    Bool,
}

impl std::fmt::Display for TensorDtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TensorDtype::F32 => write!(f, "f32"),
            TensorDtype::F64 => write!(f, "f64"),
            TensorDtype::I32 => write!(f, "i32"),
            TensorDtype::I64 => write!(f, "i64"),
            TensorDtype::Bool => write!(f, "bool"),
        }
    }
}
