use biosphere_core::ExistenceCore;

/// 基础存在
///
/// [`BasicExistence`] 是 [`ExistenceCore`] trait 的基础实现。
///
/// # 设计约束
///
/// - 基础实现：提供默认的存在行为
/// - 可覆盖：应用层可以覆盖此实现
/// - 占位符实现：类型参数是占位符，应用层应该提供具体实现
///
/// # 哲学含义
///
/// BasicExistence 是"基础存在"，而不是"具体存在"。
///
/// 这意味着：
/// - BasicExistence 提供默认的存在行为
/// - 应用层可以覆盖此实现
/// - 类型参数是占位符，应用层应该提供具体实现
#[allow(dead_code)]
pub struct BasicExistence {
    _marker: std::marker::PhantomData<()>,
}

impl BasicExistence {
    /// 创建新的基础存在
    ///
    /// # 返回值
    ///
    /// 返回新的基础存在
    ///
    /// # 设计约束
    ///
    /// - 这是默认实现
    /// - 应用层可以覆盖此实现
    /// - 类型参数是占位符
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl ExistenceCore for BasicExistence {
    type Boundary = ();
    type State = ();
    type Drive = ();
    type Rules = ();
    type Propagation = ();
}

