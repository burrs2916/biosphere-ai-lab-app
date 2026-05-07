use crate::error::{FoundationError, FoundationResult};
use crate::temporal::Tick;

/// 恢复上下文
///
/// [`RecoveryContext`] 定义了恢复操作所需的上下文信息。
///
/// # 设计约束
///
/// - 世界感知：提供世界状态信息
/// - 时间感知：提供时间信息
/// - 状态感知：提供回退能力
///
/// # 哲学含义
///
/// RecoveryContext 是"恢复操作的世界上下文"，而不是"函数执行上下文"。
///
/// 这意味着：
/// - RecoveryContext 知道世界的存在
/// - RecoveryContext 知道时间的存在
/// - RecoveryContext 可以修改世界状态
/// - RecoveryContext 是世界演化的一部分
pub trait RecoveryContext {
    /// 获取当前时间刻
    ///
    /// # 返回值
    ///
    /// 返回当前时间刻
    fn current_tick(&self) -> Tick;

    /// 检查是否可以回退
    ///
    /// # 返回值
    ///
    /// 返回 true 表示可以回退，false 表示不可以
    fn can_rollback(&self) -> bool;

    /// 回退到指定时间刻
    ///
    /// # 参数
    ///
    /// * `tick` - 要回退到的时间刻
    ///
    /// # 返回值
    ///
    /// 返回操作结果
    ///
    /// # 设计约束
    ///
    /// - 只能回退到之前的时间刻
    /// - 回退操作必须是原子的
    /// - 回退操作必须保持世界一致性
    fn rollback_to(&mut self, tick: Tick) -> FoundationResult<()>;
}

/// 世界恢复策略
///
/// [`WorldRecovery`] 定义了世界级别的恢复策略。
///
/// # 设计约束
///
/// - 世界感知：所有策略都影响世界状态
/// - 时间感知：所有策略都与时间相关
/// - 演化感知：所有策略都影响世界演化
///
/// # 哲学含义
///
/// WorldRecovery 是"世界演化中的恢复选择"，而不是"函数执行失败后的行为"。
///
/// 这意味着：
/// - WorldRecovery 知道世界的存在
/// - WorldRecovery 知道时间的存在
/// - WorldRecovery 可以改变世界演化路径
/// - WorldRecovery 是世界演化的一部分
#[derive(Debug, Clone)]
pub enum WorldRecovery {
    /// 回退到指定时间刻
    RollbackTo(Tick),

    /// 从指定时间刻分叉
    BranchFrom(Tick),

    /// 终止世界演化
    TerminateWorld,
}

/// 操作恢复策略
///
/// [`RecoveryStrategy`] 定义了操作级别的恢复策略。
///
/// # 设计约束
///
/// - 操作感知：所有策略都是针对操作的
/// - 可配置：支持不同的恢复策略
/// - 可组合：支持多个恢复策略的组合
/// - 抽象定义：不包含具体实现
///
/// # 哲学含义
///
/// RecoveryStrategy 是"操作级别的恢复策略"，而不是"世界级别的恢复策略"。
///
/// 这意味着：
/// - RecoveryStrategy 只影响单个操作
/// - RecoveryStrategy 不影响世界状态
/// - RecoveryStrategy 不影响时间演化
/// - RecoveryStrategy 是控制流的一部分，不是世界演化的一部分
///
/// # 恢复策略类型
///
/// - **忽略策略**：忽略错误，继续执行
/// - **重试策略**：重试操作，最多重试 N 次
/// - **默认策略**：使用默认值
/// - **终止策略**：终止操作，返回错误
///
/// # 注意
///
/// RecoveryStrategy 不包含回退策略，因为回退是世界级别的操作，
/// 应该使用 WorldRecovery 而不是 RecoveryStrategy。
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::recovery::{RecoveryStrategy, RecoveryResult};
///
/// let strategy = RecoveryStrategy::Retry { max_attempts: 3 };
/// // 具体的恢复逻辑由应用层实现
/// ```
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// 忽略策略
    ///
    /// 忽略错误，继续执行
    Ignore,

    /// 重试策略
    ///
    /// 重试操作，最多重试 N 次
    Retry {
        /// 最大重试次数
        max_attempts: u32,
    },

    /// 默认策略
    ///
    /// 使用默认值
    Default,

    /// 终止策略
    ///
    /// 终止操作，返回错误
    Terminate,
}

/// 恢复结果
///
/// [`RecoveryResult`] 定义了恢复操作的结果。
///
/// # 设计约束
///
/// - 类型安全：使用强类型而不是字符串
/// - 可追溯：包含恢复策略信息
/// - 演化感知：区分操作恢复和世界恢复
///
/// # 哲学含义
///
/// RecoveryResult 是"恢复操作的结果"，而不是"函数执行的结果"。
///
/// 这意味着：
/// - RecoveryResult 可以表达世界演化路径的改变
/// - RecoveryResult 可以区分操作恢复和世界恢复
/// - RecoveryResult 是世界演化的一部分
/// - RecoveryResult 可以表达"世界没有按原路径演化，但我们选择了另一条合法路径"
///
/// # 示例
///
/// ```rust
/// use biosphere_foundation::recovery::{RecoveryStrategy, RecoveryResult};
///
/// let result: RecoveryResult<()> = RecoveryResult::Ok(());
/// assert!(result.is_ok());
/// ```
#[derive(Debug, Clone)]
pub enum RecoveryResult<T> {
    /// 成功
    ///
    /// 操作成功执行，没有发生任何恢复
    Ok(T),

    /// 操作恢复成功
    ///
    /// 操作失败后，通过操作级别的恢复策略成功恢复
    OperationRecovered {
        /// 恢复后的值
        value: T,
        /// 使用的恢复策略
        strategy: RecoveryStrategy,
    },

    /// 世界恢复成功
    ///
    /// 操作失败后，通过世界级别的恢复策略成功恢复
    WorldRecovered {
        /// 使用的世界恢复策略
        recovery: WorldRecovery,
    },

    /// 恢复失败
    ///
    /// 操作失败后，所有恢复策略都失败
    Failed {
        /// 原始错误
        error: FoundationError,
        /// 使用的最后恢复策略
        strategy: RecoveryStrategy,
    },
}

impl<T> RecoveryResult<T> {
    /// 检查结果是否成功
    ///
    /// # 返回值
    ///
    /// 返回 true 表示成功，false 表示失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::recovery::RecoveryResult;
    ///
    /// let result: RecoveryResult<()> = RecoveryResult::Ok(());
    /// assert!(result.is_ok());
    /// ```
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_) | Self::OperationRecovered { .. } | Self::WorldRecovered { .. })
    }

    /// 检查结果是否失败
    ///
    /// # 返回值
    ///
    /// 返回 true 表示失败，false 表示成功
    ///
    /// # 示例
    ///
    /// ```text
    /// let result: RecoveryResult<()> = RecoveryResult::Failed {
    ///     error: FoundationError::temporal_violation("Test"),
    ///     strategy: RecoveryStrategy::Terminate,
    /// };
    /// assert!(result.is_failed());
    /// ```
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// 检查结果是否通过操作恢复成功
    ///
    /// # 返回值
    ///
    /// 返回 true 表示通过操作恢复成功，false 表示其他情况
    pub fn is_operation_recovered(&self) -> bool {
        matches!(self, Self::OperationRecovered { .. })
    }

    /// 检查结果是否通过世界恢复成功
    ///
    /// # 返回值
    ///
    /// 返回 true 表示通过世界恢复成功，false 表示其他情况
    pub fn is_world_recovered(&self) -> bool {
        matches!(self, Self::WorldRecovered { .. })
    }

    /// 获取值
    ///
    /// # 返回值
    ///
    /// 如果成功，返回值，否则返回 None
    ///
    /// # 示例
    ///
    /// ```text
    /// let result: RecoveryResult<i32> = RecoveryResult::Ok(42);
    /// assert_eq!(result.value(), Some(42));
    /// ```
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Ok(value) => Some(value),
            Self::OperationRecovered { value, .. } => Some(value),
            Self::WorldRecovered { .. } => None,
            Self::Failed { .. } => None,
        }
    }

    /// 获取世界恢复策略
    ///
    /// # 返回值
    ///
    /// 如果是世界恢复，返回恢复策略，否则返回 None
    pub fn world_recovery(&self) -> Option<&WorldRecovery> {
        match self {
            Self::WorldRecovered { recovery } => Some(recovery),
            _ => None,
        }
    }

    /// 转换为 FoundationResult
    ///
    /// # 返回值
    ///
    /// 如果成功，返回 Ok，否则返回 Err
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::recovery::RecoveryResult;
    ///
    /// let result: RecoveryResult<i32> = RecoveryResult::Ok(42);
    /// let foundation_result = result.to_foundation_result();
    /// assert!(foundation_result.is_ok());
    /// ```
    pub fn to_foundation_result(self) -> FoundationResult<T> {
        match self {
            Self::Ok(value) => Ok(value),
            Self::OperationRecovered { value, .. } => Ok(value),
            Self::WorldRecovered { .. } => Err(FoundationError::temporal_violation("World recovery cannot be converted to a value")),
            Self::Failed { error, .. } => Err(error),
        }
    }
}

impl RecoveryStrategy {
    /// 执行错误恢复
    ///
    /// # 参数
    ///
    /// * `ctx` - 恢复上下文
    /// * `operation` - 要执行的操作
    ///
    /// # 返回值
    ///
    /// 返回恢复结果
    ///
    /// # 设计约束
    ///
    /// - 这是占位实现，总是失败
    /// - 应用层必须覆盖此方法以提供实际的恢复逻辑
    /// - Foundation 层只提供接口定义
    ///
    /// # 注意
    ///
    /// 此默认实现仅用于编译通过，实际使用时应用层应该提供有意义的恢复策略。
    /// 不同的恢复策略（Ignore、Retry、Default）应该有不同的实现。
    ///
    /// # 哲学含义
    ///
    /// 这个方法明确表示：恢复操作发生在"世界里"，而不是"函数里"。
    ///
    /// # 示例
    ///
    /// ```text
    /// let strategy = RecoveryStrategy::Retry { max_attempts: 3 };
    /// // 具体的恢复逻辑由应用层实现
    /// ```
    pub fn recover<T, F>(&self, _ctx: &mut dyn RecoveryContext, operation: F) -> RecoveryResult<T>
    where
        F: FnOnce() -> FoundationResult<T>,
    {
        match operation() {
            Ok(value) => RecoveryResult::Ok(value),
            Err(error) => RecoveryResult::Failed {
                error,
                strategy: self.clone(),
            },
        }
    }
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self::Terminate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FoundationError;

    const TEST_VALUE: i32 = 42;

    /// 测试用的恢复上下文
    struct TestRecoveryContext {
        current_tick: u64,
        can_rollback: bool,
    }

    impl TestRecoveryContext {
        fn new(current_tick: u64, can_rollback: bool) -> Self {
            Self { current_tick, can_rollback }
        }
    }

    impl RecoveryContext for TestRecoveryContext {
        fn current_tick(&self) -> Tick {
            Tick::new(self.current_tick)
        }

        fn can_rollback(&self) -> bool {
            self.can_rollback
        }

        fn rollback_to(&mut self, _tick: Tick) -> FoundationResult<()> {
            Ok(())
        }
    }

    #[test]
    fn test_recovery_strategy_terminate() {
        let strategy = RecoveryStrategy::Terminate;
        let mut ctx = TestRecoveryContext::new(0, true);

        let result = strategy.recover(&mut ctx, || {
            Err::<i32, FoundationError>(FoundationError::temporal_violation("Test error"))
        });

        assert!(result.is_failed());
    }

    #[test]
    fn test_recovery_strategy_terminate_success() {
        let strategy = RecoveryStrategy::Terminate;
        let mut ctx = TestRecoveryContext::new(0, true);

        let result = strategy.recover(&mut ctx, || {
            Ok::<i32, FoundationError>(TEST_VALUE)
        });

        assert!(result.is_ok());
        assert_eq!(result.value(), Some(&TEST_VALUE));
    }

    #[test]
    fn test_recovery_result_ok() {
        let result: RecoveryResult<i32> = RecoveryResult::Ok(TEST_VALUE);
        assert!(result.is_ok());
        assert!(!result.is_failed());
        assert_eq!(result.value(), Some(&TEST_VALUE));
    }

    #[test]
    fn test_recovery_result_operation_recovered() {
        let result: RecoveryResult<i32> = RecoveryResult::OperationRecovered {
            value: TEST_VALUE,
            strategy: RecoveryStrategy::Default,
        };
        assert!(result.is_ok());
        assert!(!result.is_failed());
        assert!(result.is_operation_recovered());
        assert!(!result.is_world_recovered());
        assert_eq!(result.value(), Some(&TEST_VALUE));
    }

    #[test]
    fn test_recovery_result_world_recovered() {
        let result: RecoveryResult<i32> = RecoveryResult::WorldRecovered {
            recovery: WorldRecovery::RollbackTo(Tick::new(0)),
        };
        assert!(result.is_ok());
        assert!(!result.is_failed());
        assert!(!result.is_operation_recovered());
        assert!(result.is_world_recovered());
        assert_eq!(result.value(), None);
        assert!(result.world_recovery().is_some());
    }

    #[test]
    fn test_recovery_result_failed() {
        let result: RecoveryResult<i32> = RecoveryResult::Failed {
            error: FoundationError::temporal_violation("Test error"),
            strategy: RecoveryStrategy::Terminate,
        };
        assert!(!result.is_ok());
        assert!(result.is_failed());
        assert_eq!(result.value(), None);
    }

    #[test]
    fn test_recovery_result_to_foundation_result_ok() {
        let result: RecoveryResult<i32> = RecoveryResult::Ok(TEST_VALUE);
        let foundation_result = result.to_foundation_result();
        assert!(foundation_result.is_ok());
        assert_eq!(foundation_result.unwrap(), TEST_VALUE);
    }

    #[test]
    fn test_recovery_result_to_foundation_result_operation_recovered() {
        let result: RecoveryResult<i32> = RecoveryResult::OperationRecovered {
            value: TEST_VALUE,
            strategy: RecoveryStrategy::Default,
        };
        let foundation_result = result.to_foundation_result();
        assert!(foundation_result.is_ok());
        assert_eq!(foundation_result.unwrap(), TEST_VALUE);
    }

    #[test]
    fn test_recovery_result_to_foundation_result_world_recovered() {
        let result: RecoveryResult<i32> = RecoveryResult::WorldRecovered {
            recovery: WorldRecovery::RollbackTo(Tick::new(0)),
        };
        let foundation_result = result.to_foundation_result();
        assert!(foundation_result.is_err());
    }

    #[test]
    fn test_recovery_result_to_foundation_result_failed() {
        let result: RecoveryResult<i32> = RecoveryResult::Failed {
            error: FoundationError::temporal_violation("Test error"),
            strategy: RecoveryStrategy::Terminate,
        };
        let foundation_result = result.to_foundation_result();
        assert!(foundation_result.is_err());
    }

    #[test]
    fn test_recovery_strategy_default() {
        let strategy = RecoveryStrategy::default();
        assert!(matches!(strategy, RecoveryStrategy::Terminate));
    }
}
