use biosphere_core::ConditionSignal;
use std::collections::VecDeque;
use crate::input::input::ConditionInput;

/// 输入管理器
///
/// [`InputManager`] 管理多个条件输入源。
///
/// # 设计约束
///
/// - 中立管理：不知道具体输入源是什么
/// - 信号验证：只接受有效的信号
/// - 不包含 UI 语义：不关心输入设备类型
/// - 不包含优先级逻辑：优先级处理由应用层负责
///
/// # 哲学含义
///
/// InputManager 是"输入管理器"，而不是"输入设备管理器"。
///
/// 这意味着：
/// - InputManager 只知道 ConditionInput trait
/// - InputManager 不知道键盘、鼠标、触摸这些概念
/// - InputManager 只知道"有人给了我一些 ConditionSignal"
/// - InputManager 不处理优先级，优先级由应用层负责
///
/// # 使用场景
///
/// InputManager 用于：
/// - 管理多个输入源
/// - 验证信号有效性
/// - 为世界提供条件信号
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::input::InputManager;
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
/// let mut manager = InputManager::new();
/// let keyboard = MyInput;
/// manager.add_source(Box::new(keyboard));
///
/// let signals = manager.process();
/// ```
pub struct InputManager {
    sources: Vec<Box<dyn ConditionInput>>,
    queue: VecDeque<ConditionSignal>,
}

impl InputManager {
    /// 创建新的输入管理器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::input::InputManager;
    ///
    /// let manager = InputManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            queue: VecDeque::new(),
        }
    }

    /// 添加输入源
    ///
    /// # 参数
    ///
    /// * `source` - 要添加的输入源
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::input::InputManager;
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
    /// let mut manager = InputManager::new();
    /// let keyboard = MyInput;
    /// manager.add_source(Box::new(keyboard));
    /// ```
    pub fn add_source(&mut self, source: Box<dyn ConditionInput>) {
        self.sources.push(source);
    }

    /// 处理所有输入源
    ///
    /// # 行为
    ///
    /// 1. 收集所有输入源的信号
    /// 2. 验证信号有效性（只能由生成者验证）
    /// 3. 将有效信号加入队列
    ///
    /// # 返回值
    ///
    /// 返回生成的有效信号列表
    ///
    /// # 设计约束
    ///
    /// - 不包含优先级排序逻辑
    /// - 优先级处理由应用层负责
    /// - Foundation 层只提供数据收集接口
    /// - 信号的"生成责任"与"合法性责任"不被解耦
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::input::InputManager;
    ///
    /// let mut manager = InputManager::new();
    /// let signals = manager.process();
    /// ```
    pub fn process(&mut self) -> Vec<ConditionSignal> {
        let mut valid_signals = Vec::new();

        // 收集并验证所有输入源的信号
        // 确保每个信号只能由其生成者验证，防止跨源背书
        for source in &self.sources {
            for signal in source.generate() {
                if source.validate_own(&signal) {
                    self.queue.push_back(signal.clone());
                    valid_signals.push(signal);
                }
            }
        }

        valid_signals
    }

    /// 获取队列中的下一个信号
    ///
    /// # 返回值
    ///
    /// 如果队列不为空，返回下一个信号，否则返回 None
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::input::InputManager;
    ///
    /// let mut manager = InputManager::new();
    /// let next_signal = manager.next();
    /// ```
    pub fn next(&mut self) -> Option<ConditionSignal> {
        self.queue.pop_front()
    }

    /// 获取队列中的所有信号
    ///
    /// # 返回值
    ///
    /// 返回队列中的所有信号
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::input::InputManager;
    ///
    /// let manager = InputManager::new();
    /// let all_signals = manager.peek_all();
    /// ```
    pub fn peek_all(&self) -> Vec<&ConditionSignal> {
        self.queue.iter().collect()
    }

    /// 检查队列是否为空
    ///
    /// # 返回值
    ///
    /// 如果队列为空，返回 true，否则返回 false
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::input::InputManager;
    ///
    /// let manager = InputManager::new();
    /// let is_empty = manager.is_empty();
    /// ```
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockInput {
        signals: Vec<ConditionSignal>,
    }

    impl ConditionInput for MockInput {
        fn generate(&self) -> Vec<ConditionSignal> {
            self.signals.clone()
        }

        fn validate_own(&self, _signal: &ConditionSignal) -> bool {
            true
        }
    }

    #[test]
    fn test_input_manager_creation() {
        let manager = InputManager::new();
        assert!(manager.is_empty());
        assert!(manager.peek_all().is_empty());
    }

    #[test]
    fn test_input_manager_add_source() {
        let mut manager = InputManager::new();
        let input = MockInput {
            signals: vec![],
        };
        manager.add_source(Box::new(input));
        assert!(!manager.sources.is_empty());
    }

    #[test]
    fn test_input_manager_process() {
        let mut manager = InputManager::new();
        let input = MockInput {
            signals: vec![],
        };
        manager.add_source(Box::new(input));
        let signals = manager.process();
        assert!(signals.is_empty());
    }

    #[test]
    fn test_input_manager_next() {
        let mut manager = InputManager::new();
        let input = MockInput {
            signals: vec![],
        };
        manager.add_source(Box::new(input));
        let signals = manager.process();
        assert!(signals.is_empty());

        let next = manager.next();
        assert!(next.is_none());
    }
}
