use crate::manifest::{Manifest, ManifestNode, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum DiffOperation {
    Insert {
        path: Vec<usize>,
        node: ManifestNode,
    },
    Update {
        path: Vec<usize>,
        old_value: Value,
        new_value: Value,
    },
    Delete {
        path: Vec<usize>,
        node: ManifestNode,
    },
    Move {
        from_path: Vec<usize>,
        to_path: Vec<usize>,
        node: ManifestNode,
    },
}

pub trait ManifestDiff {
    fn diff(&self, other: &Manifest) -> Vec<DiffOperation>;
}
