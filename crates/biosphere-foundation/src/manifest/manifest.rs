use crate::manifest::ManifestNode;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub time: u64,
    pub root: ManifestNode,
}

impl Manifest {
    pub fn new(time: u64, root: ManifestNode) -> Self {
        Manifest { time, root }
    }

    pub fn time(&self) -> u64 {
        self.time
    }

    pub fn root(&self) -> &ManifestNode {
        &self.root
    }

    pub fn into_root(self) -> ManifestNode {
        self.root
    }
}

impl fmt::Display for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Manifest @ tick {}", self.time)?;
        write!(f, "{}", self.root)
    }
}
