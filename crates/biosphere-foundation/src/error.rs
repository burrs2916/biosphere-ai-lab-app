/// Foundation 层错误类型
///
/// [`FoundationError`] 定义了 Foundation 层的所有错误类型。
///
/// # 设计约束
///
/// - 类型安全：使用枚举而不是字符串
/// - 可恢复：所有错误都应该可以恢复
/// - 可追溯：错误应该包含足够的上下文信息
///
/// # 哲学含义
///
/// FoundationError 是"Foundation 层的错误"，而不是"系统错误"。
///
/// 这意味着：
/// - 错误是可预期的
/// - 错误是可恢复的
/// - 错误不应该导致系统崩溃
///
/// # 错误分类
///
/// - **时间错误**：时间相关的错误
/// - **状态错误**：状态相关的错误
/// - **关系错误**：关系相关的错误
/// - **命令错误**：命令相关的错误
/// - **查询错误**：查询相关的错误
///
/// # 示例
///
/// ```text
/// match result {
///     Ok(_) => println!("Success"),
///     Err(FoundationError::TemporalViolation { message }) => {
///         println!("Temporal violation: {}", message);
///     }
///     Err(e) => println!("Error: {:?}", e),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundationError {
    /// 时间违规错误
    ///
    /// 当时间操作违反时间公理时发生
    TemporalViolation {
        /// 错误消息
        message: String,
    },

    /// 状态错误
    ///
    /// 当状态操作失败时发生
    StateError {
        /// 错误消息
        message: String,
    },

    /// 关系错误
    ///
    /// 当关系操作失败时发生
    RelationError {
        /// 错误消息
        message: String,
    },

    /// 命令错误
    ///
    /// 当命令执行失败时发生
    CommandError {
        /// 错误消息
        message: String,
    },

    /// 查询错误
    ///
    /// 当查询操作失败时发生
    QueryError {
        /// 错误消息
        message: String,
    },

    /// 游标错误
    ///
    /// 当游标操作失败时发生
    CursorError {
        /// 错误消息
        message: String,
    },
}

impl FoundationError {
    /// 创建时间违规错误
    ///
    /// # 参数
    ///
    /// * `message` - 错误消息
    ///
    /// # 返回值
    ///
    /// 返回新的时间违规错误
    ///
    /// # 示例
    ///
    /// ```text
    /// let error = FoundationError::temporal_violation("Time cannot go backwards");
    /// ```
    pub fn temporal_violation(message: impl Into<String>) -> Self {
        Self::TemporalViolation {
            message: message.into(),
        }
    }

    /// 创建状态错误
    ///
    /// # 参数
    ///
    /// * `message` - 错误消息
    ///
    /// # 返回值
    ///
    /// 返回新的状态错误
    ///
    /// # 示例
    ///
    /// ```text
    /// let error = FoundationError::state_error("State not found");
    /// ```
    pub fn state_error(message: impl Into<String>) -> Self {
        Self::StateError {
            message: message.into(),
        }
    }

    /// 创建关系错误
    ///
    /// # 参数
    ///
    /// * `message` - 错误消息
    ///
    /// # 返回值
    ///
    /// 返回新的关系错误
    ///
    /// # 示例
    ///
    /// ```text
    /// let error = FoundationError::relation_error("Relation not found");
    /// ```
    pub fn relation_error(message: impl Into<String>) -> Self {
        Self::RelationError {
            message: message.into(),
        }
    }

    /// 创建命令错误
    ///
    /// # 参数
    ///
    /// * `message` - 错误消息
    ///
    /// # 返回值
    ///
    /// 返回新的命令错误
    ///
    /// # 示例
    ///
    /// ```text
    /// let error = FoundationError::command_error("Command execution failed");
    /// ```
    pub fn command_error(message: impl Into<String>) -> Self {
        Self::CommandError {
            message: message.into(),
        }
    }

    /// 创建查询错误
    ///
    /// # 参数
    ///
    /// * `message` - 错误消息
    ///
    /// # 返回值
    ///
    /// 返回新的查询错误
    ///
    /// # 示例
    ///
    /// ```text
    /// let error = FoundationError::query_error("Query failed");
    /// ```
    pub fn query_error(message: impl Into<String>) -> Self {
        Self::QueryError {
            message: message.into(),
        }
    }

    /// 创建游标错误
    ///
    /// # 参数
    ///
    /// * `message` - 错误消息
    ///
    /// # 返回值
    ///
    /// 返回新的游标错误
    ///
    /// # 示例
    ///
    /// ```text
    /// let error = FoundationError::cursor_error("Cannot move cursor beyond bounds");
    /// ```
    pub fn cursor_error(message: impl Into<String>) -> Self {
        Self::CursorError {
            message: message.into(),
        }
    }

    /// 检查错误是否可恢复
    ///
    /// # 返回值
    ///
    /// 返回 true 表示错误可恢复，false 表示不可恢复
    ///
    /// # 示例
    ///
    /// ```text
    /// let error = FoundationError::temporal_violation("Time cannot go backwards");
    /// assert!(error.is_recoverable());
    /// ```
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::TemporalViolation { .. } => true,
            Self::StateError { .. } => true,
            Self::RelationError { .. } => true,
            Self::CommandError { .. } => true,
            Self::QueryError { .. } => true,
            Self::CursorError { .. } => true,
        }
    }

    /// 获取错误消息
    ///
    /// # 返回值
    ///
    /// 返回错误消息
    ///
    /// # 示例
    ///
    /// ```text
    /// let error = FoundationError::temporal_violation("Time cannot go backwards");
    /// assert_eq!(error.message(), "Time cannot go backwards");
    /// ```
    pub fn message(&self) -> &str {
        match self {
            Self::TemporalViolation { message } => message,
            Self::StateError { message } => message,
            Self::RelationError { message } => message,
            Self::CommandError { message } => message,
            Self::QueryError { message } => message,
            Self::CursorError { message } => message,
        }
    }
}

impl std::fmt::Display for FoundationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TemporalViolation { message } => write!(f, "Temporal violation: {}", message),
            Self::StateError { message } => write!(f, "State error: {}", message),
            Self::RelationError { message } => write!(f, "Relation error: {}", message),
            Self::CommandError { message } => write!(f, "Command error: {}", message),
            Self::QueryError { message } => write!(f, "Query error: {}", message),
            Self::CursorError { message } => write!(f, "Cursor error: {}", message),
        }
    }
}

impl std::error::Error for FoundationError {}

/// Foundation 层结果类型
///
/// [`FoundationResult`] 是 Foundation 层的标准结果类型。
///
/// # 设计约束
///
/// - 类型安全：使用强类型而不是字符串
/// - 可恢复：所有错误都应该可以恢复
/// - 可追溯：错误应该包含足够的上下文信息
///
/// # 示例
///
/// ```text
/// fn do_something() -> FoundationResult<()> {
///     Err(FoundationError::temporal_violation("Time cannot go backwards"))
/// }
/// ```
pub type FoundationResult<T> = Result<T, FoundationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_violation() {
        let error = FoundationError::temporal_violation("Time cannot go backwards");
        assert_eq!(error.message(), "Time cannot go backwards");
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_state_error() {
        let error = FoundationError::state_error("State not found");
        assert_eq!(error.message(), "State not found");
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_relation_error() {
        let error = FoundationError::relation_error("Relation not found");
        assert_eq!(error.message(), "Relation not found");
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_command_error() {
        let error = FoundationError::command_error("Command execution failed");
        assert_eq!(error.message(), "Command execution failed");
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_query_error() {
        let error = FoundationError::query_error("Query failed");
        assert_eq!(error.message(), "Query failed");
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_cursor_error() {
        let error = FoundationError::cursor_error("Cannot move cursor beyond bounds");
        assert_eq!(error.message(), "Cannot move cursor beyond bounds");
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_error_display() {
        let error = FoundationError::temporal_violation("Time cannot go backwards");
        let display = format!("{}", error);
        assert!(display.contains("Temporal violation"));
        assert!(display.contains("Time cannot go backwards"));
    }

    #[test]
    fn test_error_debug() {
        let error = FoundationError::temporal_violation("Time cannot go backwards");
        let debug = format!("{:?}", error);
        assert!(debug.contains("TemporalViolation"));
    }

    #[test]
    fn test_error_clone() {
        let error = FoundationError::temporal_violation("Time cannot go backwards");
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_error_equality() {
        let error1 = FoundationError::temporal_violation("Time cannot go backwards");
        let error2 = FoundationError::temporal_violation("Time cannot go backwards");
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_foundation_result() {
        let result: FoundationResult<()> = Err(FoundationError::temporal_violation("Time cannot go backwards"));
        assert!(result.is_err());
    }
}
