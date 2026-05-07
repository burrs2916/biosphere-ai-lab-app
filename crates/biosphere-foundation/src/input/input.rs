use biosphere_core::ConditionSignal;

/// 条件输入接口
///
/// 这个 trait 定义了如何生成条件信号，但不指定信号来源。
/// 具体的输入源（如键盘、鼠标、触摸）应该在 biosphere-ui 层实现。
///
/// # 设计约束
///
/// - 中立接口：不包含任何设备语义
/// - 不指定来源：只定义如何生成信号
/// - 可验证：提供信号验证能力
///
/// # 哲学含义
///
/// ConditionInput 是"条件输入接口"，而不是"输入设备"。
///
/// 这意味着：
/// - Foundation 不知道信号来自哪里
/// - Foundation 只知道"有人给了我一些 ConditionSignal"
/// - 具体的输入源在 UI 层实现
///
/// # 使用场景
///
/// ConditionInput 用于：
/// - UI 层将输入转换为条件信号
/// - InputManager 管理多个输入源
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::input::ConditionInput;
/// use biosphere_core::ConditionSignal;
///
/// struct KeyboardInput {
///     // 具体实现细节
/// }
///
/// impl ConditionInput for KeyboardInput {
///     fn generate(&self) -> Vec<ConditionSignal> {
///         // 生成条件信号
///         vec![]
///     }
///
///     fn validate_own(&self, signal: &ConditionSignal) -> bool {
///         // 验证信号
///         true
///     }
/// }
/// ```
pub trait ConditionInput {
    /// 生成条件信号
    ///
    /// # 返回值
    ///
    /// 返回生成的条件信号列表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::input::ConditionInput;
    /// use biosphere_core::ConditionSignal;
    ///
    /// struct MyInput;
    ///
    /// impl ConditionInput for MyInput {
///     fn generate(&self) -> Vec<ConditionSignal> {
///         vec![]
///     }
///
///     fn validate_own(&self, _signal: &ConditionSignal) -> bool {
///         true
///     }
/// }
    ///
    /// let input = MyInput;
    /// let signals = input.generate();
    /// ```
    fn generate(&self) -> Vec<ConditionSignal>;

    /// 验证条件信号
    ///
    /// # 参数
    ///
    /// * `signal` - 要验证的条件信号
    ///
    /// # 返回值
    ///
    /// 如果信号有效，返回 true，否则返回 false
    ///
    /// # 语义约束
    ///
    /// 此方法只能验证由自身生成的信号，不能验证其他输入源生成的信号。
    /// 这确保了信号的"生成责任"与"合法性责任"不被解耦。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::input::ConditionInput;
    /// use biosphere_core::ConditionSignal;
    ///
    /// struct MyInput;
    ///
    /// impl ConditionInput for MyInput {
    ///     fn generate(&self) -> Vec<ConditionSignal> {
    ///         vec![]
    ///     }
    ///
    ///     fn validate_own(&self, _signal: &ConditionSignal) -> bool {
    ///         true
    ///     }
    /// }
    ///
    /// let input = MyInput;
    /// let signal = ConditionSignal {
    ///         kind: "test",
    ///         intensity: 0,
    ///     };
    /// let is_valid = input.validate_own(&signal);
    /// ```
    fn validate_own(&self, signal: &ConditionSignal) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockInput;

    impl ConditionInput for MockInput {
        fn generate(&self) -> Vec<ConditionSignal> {
            vec![]
        }

        fn validate_own(&self, _signal: &ConditionSignal) -> bool {
            true
        }
    }

    #[test]
    fn test_condition_input_generate() {
        let input = MockInput;
        let signals = input.generate();
        assert!(signals.is_empty());
    }

    #[test]
    fn test_condition_input_validate() {
        let input = MockInput;
        let signal = ConditionSignal {
            kind: "test",
            intensity: 0,
        };
        assert!(input.validate_own(&signal));
    }
}
