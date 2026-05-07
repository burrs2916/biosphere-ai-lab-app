/// Manifest 路径
///
/// [`ManifestPath`] 是 Manifest 中节点的路径表示。
///
/// # 设计约束
///
/// - 索引序列：使用 usize 索引序列表示路径
/// - 从根开始：路径从根节点开始
/// - 零基：索引从 0 开始
///
/// # 哲学含义
///
/// ManifestPath 是"在 Manifest 中定位节点的方式"，而不是"节点标识符"。
///
/// 这意味着：
/// - ManifestPath 是相对路径，不是绝对标识
/// - ManifestPath 依赖于 Manifest 结构
/// - ManifestPath 可以用于遍历和访问
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::perception::ManifestPath;
///
/// // 表示根节点的第一个子节点的第二个子节点
/// let path = ManifestPath::from(vec![0, 1]);
/// ```
pub type ManifestPath = Vec<usize>;

/// Manifest 路径扩展方法
pub trait ManifestPathExt {
    /// 创建新的路径
    ///
    /// # 参数
    ///
    /// * `path` - 索引序列
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::perception::{ManifestPath, ManifestPathExt};
    ///
    /// let path = ManifestPath::new(vec![0, 1, 2]);
    /// ```
    fn new(path: Vec<usize>) -> Self;

    /// 获取路径的最后一个索引
    ///
    /// # 返回值
    ///
    /// 返回路径的最后一个索引，如果路径为空则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::perception::{ManifestPath, ManifestPathExt};
    ///
    /// let path = ManifestPath::new(vec![0, 1, 2]);
    /// assert_eq!(path.last(), Some(&2));
    ///
    /// let empty_path = ManifestPath::new(vec![]);
    /// assert_eq!(empty_path.last(), None);
    /// ```
    fn last(&self) -> Option<&usize>;

    /// 获取路径的长度
    ///
    /// # 返回值
    ///
    /// 返回路径的长度
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::perception::{ManifestPath, ManifestPathExt};
    ///
    /// let path = ManifestPath::new(vec![0, 1, 2]);
    /// assert_eq!(path.len(), 3);
    /// ```
    fn len(&self) -> usize;

    /// 检查路径是否为空
    ///
    /// # 返回值
    ///
    /// 返回 true 表示路径为空，false 表示路径非空
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::perception::{ManifestPath, ManifestPathExt};
    ///
    /// let empty_path = ManifestPath::new(vec![]);
    /// assert!(empty_path.is_empty());
    ///
    /// let path = ManifestPath::new(vec![0, 1]);
    /// assert!(!path.is_empty());
    /// ```
    fn is_empty(&self) -> bool;

    /// 在路径末尾添加索引
    ///
    /// # 参数
    ///
    /// * `index` - 要添加的索引
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::perception::{ManifestPath, ManifestPathExt};
    ///
    /// let mut path = ManifestPath::new(vec![0, 1]);
    /// path.push(2);
    /// assert_eq!(path, vec![0, 1, 2]);
    /// ```
    fn push(&mut self, index: usize);

    /// 从路径末尾移除索引
    ///
    /// # 返回值
    ///
    /// 返回被移除的索引，如果路径为空则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::perception::{ManifestPath, ManifestPathExt};
    ///
    /// let mut path = ManifestPath::new(vec![0, 1, 2]);
    /// assert_eq!(path.pop(), Some(2));
    /// assert_eq!(path, vec![0, 1]);
    ///
    /// let mut empty_path = ManifestPath::new(vec![]);
    /// assert_eq!(empty_path.pop(), None);
    /// ```
    fn pop(&mut self) -> Option<usize>;
}

impl ManifestPathExt for ManifestPath {
    fn new(path: Vec<usize>) -> Self {
        path
    }

    fn last(&self) -> Option<&usize> {
        if self.is_empty() {
            None
        } else {
            Some(&self[self.len() - 1])
        }
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }

    fn push(&mut self, index: usize) {
        Vec::push(self, index);
    }

    fn pop(&mut self) -> Option<usize> {
        Vec::pop(self)
    }
}