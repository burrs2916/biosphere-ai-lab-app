use crate::manifest::Value;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum NodeKind {
    Scalar,
    Group,
    Sequence,
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeKind::Scalar => write!(f, "Scalar"),
            NodeKind::Group => write!(f, "Group"),
            NodeKind::Sequence => write!(f, "Sequence"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestNode {
    pub kind: NodeKind,
    pub value: Value,
    pub children: Vec<ManifestNode>,
}

impl ManifestNode {
    pub fn scalar(value: Value) -> Self {
        ManifestNode {
            kind: NodeKind::Scalar,
            value,
            children: Vec::new(),
        }
    }

    pub fn group(value: Value, children: Vec<ManifestNode>) -> Self {
        ManifestNode {
            kind: NodeKind::Group,
            value,
            children,
        }
    }

    pub fn sequence(value: Value, children: Vec<ManifestNode>) -> Self {
        ManifestNode {
            kind: NodeKind::Sequence,
            value,
            children,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }
    
    /// 获取节点类型
    pub fn kind(&self) -> NodeKind {
        self.kind
    }
    
    /// 获取节点值
    pub fn value(&self) -> &Value {
        &self.value
    }
    
    /// 获取子节点
    pub fn children(&self) -> &[ManifestNode] {
        &self.children
    }
}

impl fmt::Display for ManifestNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl ManifestNode {
    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let indent_str = "  ".repeat(indent);
        write!(f, "{}[{}] {}", indent_str, self.kind, self.value)?;
        
        if !self.children.is_empty() {
            writeln!(f)?;
            for child in &self.children {
                child.fmt_with_indent(f, indent + 1)?;
            }
        } else {
            writeln!(f)?;
        }
        Ok(())
    }
}
