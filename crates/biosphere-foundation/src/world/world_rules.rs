use biosphere_core::ExistentialTopology;

/// 世界规则
///
/// [`WorldRules`] 描述世界的约束，调用公理来验证迁移的有效性。
///
/// # 设计约束
///
/// - 不可破坏：世界规则不能被破坏
/// - 原子性：规则验证是原子的
/// - 可扩展：可以添加新的规则
///
/// # 哲学含义
///
/// WorldRules 是"世界的法律"，定义了哪些迁移是有效的。
///
/// 这意味着：
/// - 任何违反规则的迁移都会被拒绝
/// - 规则验证是原子的，不会出现部分验证
/// - 规则可以扩展，但不能破坏现有规则
#[derive(Clone)]
pub struct WorldRules<T>
where
    T: ExistentialTopology,
{
    _phantom: std::marker::PhantomData<T>,
}

impl<T> WorldRules<T>
where
    T: ExistentialTopology,
{
    /// 创建新的世界规则
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// 验证迁移是否有效
    ///
    /// # 返回值
    ///
    /// 返回 true 表示迁移有效，false 表示无效
    ///
    /// # 设计约束
    ///
    /// - 这是基础实现，总是返回 true
    /// - 应用层可以覆盖此方法来实现自定义规则
    /// - Foundation 层不包含具体的规则逻辑
    pub fn validate(&self, _current: &T, _target: &T) -> bool {
        true
    }
}

impl<T> Default for WorldRules<T>
where
    T: ExistentialTopology,
{
    fn default() -> Self {
        Self::new()
    }
}
