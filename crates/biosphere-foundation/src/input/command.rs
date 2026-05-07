use crate::world::basic_world::BasicWorld;

/// 命令
///
/// [`Command`] 定义了命令的接口。
///
/// # 设计约束
///
/// - 单向执行：只提供 `apply` 方法，不提供 `undo` 和 `redo` 方法
/// - 时间驱动：Command 只负责产生新的 StateSnapshot / RelationChange
/// - 业务承载：Command 承担业务爆炸，Intent 保持稳定
///
/// # 哲学含义
///
/// Command 是"请求执行"，而不是"可撤销的操作"。
///
/// 这意味着：
/// - undo/redo 是 WorldRuntime 的时间游标行为，不是 command 的职责
/// - undo ≠ 反向操作
/// - undo = 改变观察 tick
/// - redo = 改变观察 tick
/// - 世界历史不应该被修改
/// - Command 只负责产生新的 StateSnapshot / RelationChange
///
/// # 为什么不包含 undo 和 redo？
///
/// 在传统的命令模式中，Command 通常包含 `execute`、`undo` 和 `redo` 方法。
/// 但在我们的时间驱动系统中，这种设计是错误的。
///
/// 原因：
/// 1. **时间不可逆**：世界历史是 append-only 的，不能被修改
/// 2. **时间游标**：undo/redo 是 WorldRuntime 的时间游标行为
/// 3. **单向执行**：Command 只负责产生新的 StateSnapshot / RelationChange
///
/// 正确的做法：
/// - Command 只包含 `apply` 方法
/// - WorldRuntime 管理时间游标
/// - undo/redo 通过改变观察 tick 实现
///
/// # 示例
///
/// 正确的 Command 设计：
/// ```text
/// pub trait Command {
///     fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String>;
/// }
/// ```
///
/// 错误的 Command 设计（包含 undo 和 redo）：
/// ```text
/// pub trait Command {
///     fn execute(&self, runtime: &mut BasicWorld) -> Result<(), String>;
///     fn undo(&self, runtime: &mut BasicWorld) -> Result<(), String>;
///     fn redo(&self, runtime: &mut BasicWorld) -> Result<(), String>;
/// }
/// ```
///
/// # 注意
///
/// **不要在 Command trait 中添加 undo 和 redo 方法！**
///
/// 这是一个哲学级别的修正，不是结构性重写。
/// undo/redo 是 runtime 时间视角，不是 command 的职责。
pub trait Command {
    /// 应用命令
    ///
    /// # 参数
    ///
    /// * `runtime` - 世界运行时
    ///
    /// # 返回值
    ///
    /// 如果命令应用成功，返回 Ok(())，否则返回错误
    ///
    /// # 行为
    ///
    /// 这个方法应该：
    /// 1. 修改世界状态
    /// 2. 产生新的 StateSnapshot
    /// 3. 产生新的 RelationChange（如果需要）
    ///
    /// # 注意
    ///
    /// - 这个方法不应该修改世界历史
    /// - 这个方法不应该实现 undo 逻辑
    /// - undo/redo 是 WorldRuntime 的时间游标行为
    fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::basic_world::BasicWorld;
    use crate::temporal::Tick;

    #[derive(Debug, Clone)]
    struct TestCommand;

    impl Command for TestCommand {
        fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String> {
            runtime.step_world().map_err(|e| format!("World step failed: {:?}", e))?;
            Ok(())
        }
    }

    #[test]
    fn test_command_apply() {
        let mut world = BasicWorld::new();
        let command = TestCommand;

        let result = command.apply(&mut world);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_multiple_apply() {
        let mut world = BasicWorld::new();
        let command = TestCommand;

        for _ in 0..3 {
            let result = command.apply(&mut world);
            assert!(result.is_ok());
        }

        assert_eq!(world.environment().current_tick(), Tick::new(3));
    }
}
