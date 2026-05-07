use std::any::Any;
use std::sync::Arc;

/// 状态内容（中立容器）
///
/// ❗Foundation 不理解 payload 的含义
///
/// # 设计约束
///
/// - 中立容器：Foundation 不理解 payload 的含义
/// - 类型安全：使用 Any + Send + Sync 保证类型安全
/// - 不可构造：外部代码无法构造 StatePayload（只能通过 new）
/// - 只读访问：只提供 downcast_ref，不提供 downcast_mut
///
/// # 哲学含义
///
/// StatePayload 是"状态内容（中立容器）"，而不是"可解释的状态"。
///
/// 这意味着：
/// - Foundation 不解释状态
/// - UI 不能构造状态
/// - 生命不能反推世界
/// - 这是整个系统抗污染的关键器官
#[derive(Debug, Clone)]
pub struct StatePayload {
    inner: Arc<dyn Any + Send + Sync>,
}

impl StatePayload {
    /// 创建新的状态载荷
    ///
    /// # 类型参数
    ///
    /// * `T` - 状态值的类型，必须实现 Any + Send + Sync
    ///
    /// # 参数
    ///
    /// * `value` - 状态值
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::state::StatePayload;
    ///
    /// let payload = StatePayload::new("hello world");
    /// ```
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self {
            inner: Arc::new(value),
        }
    }

    /// 尝试将载荷转换为指定类型的引用
    ///
    /// # 类型参数
    ///
    /// * `T` - 目标类型
    ///
    /// # 返回值
    ///
    /// 如果载荷的类型与 T 匹配，返回 Some(&T)，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::temporal::state::StatePayload;
    ///
    /// let payload = StatePayload::new(42i32);
    /// if let Some(value) = payload.downcast_ref::<i32>() {
    ///     assert_eq!(*value, 42);
    /// }
    /// ```
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_payload_creation() {
        let payload = StatePayload::new(42i32);
        assert!(payload.downcast_ref::<i32>().is_some());
    }

    #[test]
    fn test_state_payload_downcast() {
        let payload = StatePayload::new("hello");
        let value = payload.downcast_ref::<&str>();
        assert_eq!(value, Some(&"hello"));
    }

    #[test]
    fn test_state_payload_downcast_wrong_type() {
        let payload = StatePayload::new(42i32);
        let value = payload.downcast_ref::<&str>();
        assert!(value.is_none());
    }

    #[test]
    fn test_state_payload_clone() {
        let payload = StatePayload::new(42i32);
        let cloned = payload.clone();
        
        assert_eq!(
            payload.downcast_ref::<i32>(),
            cloned.downcast_ref::<i32>()
        );
    }
}