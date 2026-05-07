use crate::world::BasicWorld;
use crate::input::Command;
use crate::temporal::{StateQuery, RelationQuery, StateSnapshot, RelationChange, Tick};

/// 运行时模式
///
/// [`RuntimeMode`] 定义了运行时的模式。
///
/// # 设计约束
///
/// - 权威性：区分是否有权执行命令
/// - 可观察性：区分是否只读
/// - 明确性：所有模式都显式声明
///
/// # 哲学含义
///
/// RuntimeMode 是"运行时的权限声明"，而不是"功能开关"。
///
/// 这意味着：
/// - RuntimeMode 明确声明了运行时的能力边界
/// - RuntimeMode 防止了隐含的假设
/// - RuntimeMode 是未来多 Runtime / 多 Agent 的基础
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeMode {
    /// 权威模式
    ///
    /// 拥有世界推进权，可以执行命令
    Authoritative,
    
    /// 观察模式
    ///
    /// 只读 + 游标，不能执行命令
    Observational,
}

/// 观察游标
///
/// [`ObservationCursor`] 定义了观察游标类型。
///
/// # 设计约束
///
/// - 类型安全：使用强类型而不是原始值
/// - 语义明确：明确表示观察视角
/// - 可扩展：支持未来的多观察者需求
///
/// # 哲学含义
///
/// ObservationCursor 是"观察者的时间视角"，而不是"世界的时间"。
///
/// 这意味着：
/// - ObservationCursor 明确了主观视角的概念
/// - ObservationCursor 是多观察者 / 多代理的基础
/// - ObservationCursor 区分了客观时间和主观视角
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObservationCursor(pub u64);

/// 世界运行时
///
/// [`WorldRuntime`] 管理世界的运行时状态，包括时间游标和命令执行。
///
/// # 设计约束
///
/// - 时间游标：管理当前观察的时间刻
/// - 命令执行：提供命令执行接口
/// - 无 undo/redo：不包含 undo/redo 逻辑（UI 特定概念）
/// - 无命令队列：不包含命令队列，命令队列由 UI 层管理
/// - 模式感知：明确运行时的能力和权限
///
/// # 哲学含义
///
/// WorldRuntime 是"世界的运行时"，而不是"UI 的控制器"。
///
/// 这意味着：
/// - WorldRuntime 管理时间游标，不关心 UI
/// - WorldRuntime 提供命令执行接口，不管理命令队列
/// - WorldRuntime 不依赖 UI 框架
/// - WorldRuntime 不包含 undo/redo 逻辑（UI 特定概念）
/// - WorldRuntime 明确声明自己的能力和权限
///
/// # 与 UI 层的关系
///
/// - UI 层管理命令队列
/// - UI 层调用 WorldRuntime::execute_command 执行命令
/// - UI 层通过 set_cursor 实现 undo/redo
/// - WorldRuntime 不关心命令的来源
pub struct WorldRuntime {
    world: BasicWorld,
    cursor: ObservationCursor,
    mode: RuntimeMode,
}

impl WorldRuntime {
    /// 创建新的世界运行时
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::runtime::WorldRuntime;
    ///
    /// let runtime = WorldRuntime::new();
    /// ```
    pub fn new() -> Self {
        let world = BasicWorld::new();
        let cursor = ObservationCursor(0);
        let mode = RuntimeMode::Authoritative;

        Self { world, cursor, mode }
    }

    /// 获取当前观察的时间刻
    ///
    /// # 返回值
    ///
    /// 返回当前观察的时间刻
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::runtime::WorldRuntime;
    ///
    /// let runtime = WorldRuntime::new();
    /// let cursor = runtime.cursor();
    /// ```
    pub fn cursor(&self) -> u64 {
        self.cursor.0
    }

    /// 设置观察的时间刻
    ///
    /// # 参数
    ///
    /// * `tick` - 要观察的时间刻
    ///
    /// # 错误
    ///
    /// 如果时间刻超出历史范围，返回错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::runtime::WorldRuntime;
    ///
    /// let mut runtime = WorldRuntime::new();
    /// runtime.set_cursor(0).expect("Failed to set cursor");
    /// ```
    pub fn set_cursor(&mut self, tick: u64) -> Result<(), String> {
        let latest_tick = self.world.latest_tick();

        if tick > latest_tick {
            return Err(format!(
                "Cannot set cursor to tick {}: exceeds latest tick {}",
                tick, latest_tick
            ));
        }

        self.cursor = ObservationCursor(tick);
        Ok(())
    }

    /// 执行命令
    ///
    /// # 参数
    ///
    /// * `command` - 要执行的命令
    ///
    /// # 行为
    ///
    /// 1. 检查运行时模式
    /// 2. 执行命令
    /// 3. 推进世界时间
    /// 4. 将游标移动到最新时间刻
    ///
    /// # 错误
    ///
    /// 如果命令执行失败，返回错误
    /// 如果运行时不是权威模式，返回错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::runtime::WorldRuntime;
    /// use biosphere_foundation::input::Command;
    /// use biosphere_foundation::world::BasicWorld;
    ///
    /// struct TestCommand;
    ///
    /// impl Command for TestCommand {
    ///     fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut runtime = WorldRuntime::new();
    /// let command = TestCommand;
    /// runtime.execute_command(&command).expect("Failed to execute command");
    /// ```
    pub fn execute_command(&mut self, command: &dyn Command) -> Result<(), String> {
        // 检查运行时模式
        if self.mode != RuntimeMode::Authoritative {
            return Err("Runtime is not authoritative".into());
        }

        // 执行命令
        command.apply(&mut self.world)?;
        
        // 推进世界时间并更新游标
        let latest_tick = self.world.latest_tick();
        self.cursor = ObservationCursor(latest_tick);

        Ok(())
    }

    /// 获取当前观察的状态快照
    ///
    /// # 返回值
    ///
    /// 返回当前观察的时间刻的状态快照
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::runtime::WorldRuntime;
    ///
    /// let runtime = WorldRuntime::new();
    /// let snapshot = runtime.current_snapshot();
    /// ```
    pub fn current_snapshot(&self) -> Option<&StateSnapshot> {
        self.world.get_at(Tick::new(self.cursor.0))
    }

    /// 获取世界引用
    ///
    /// # 返回值
    ///
    /// 返回世界的引用
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::runtime::WorldRuntime;
    ///
    /// let runtime = WorldRuntime::new();
    /// let world = runtime.world();
    /// ```
    pub fn world(&self) -> &BasicWorld {
        &self.world
    }

    /// 获取运行时模式
    ///
    /// # 返回值
    ///
    /// 返回当前的运行时模式
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::runtime::{WorldRuntime, RuntimeMode};
    ///
    /// let runtime = WorldRuntime::new();
    /// let mode = runtime.mode();
    /// assert_eq!(mode, &RuntimeMode::Authoritative);
    /// ```
    pub fn mode(&self) -> &RuntimeMode {
        &self.mode
    }

    /// 设置运行时模式
    ///
    /// # 参数
    ///
    /// * `mode` - 要设置的运行时模式
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::runtime::{WorldRuntime, RuntimeMode};
    ///
    /// let mut runtime = WorldRuntime::new();
    /// runtime.set_mode(RuntimeMode::Observational);
    /// ```
    pub fn set_mode(&mut self, mode: RuntimeMode) {
        self.mode = mode;
    }
}

impl StateQuery for WorldRuntime {
    fn get_at(&self, tick: Tick) -> Option<&StateSnapshot> {
        self.world.get_at(tick)
    }

    fn query_range(&self, start: Tick, end: Tick) -> Vec<&StateSnapshot> {
        self.world.query_range(start, end)
    }

    fn latest_snapshot(&self) -> Option<&StateSnapshot> {
        self.world.latest_snapshot()
    }
}

impl RelationQuery for WorldRuntime {
    fn get_relation_at(&self, tick: Tick) -> Option<&RelationChange> {
        self.world.get_relation_at(tick)
    }

    fn query_relations_range(&self, start: Tick, end: Tick) -> Vec<&RelationChange> {
        self.world.query_relations_range(start, end)
    }

    fn latest_relation_change(&self) -> Option<&RelationChange> {
        self.world.latest_relation_change()
    }
}

impl Default for WorldRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::BasicWorld;

    #[derive(Debug, Clone)]
    struct TestCommand;

    impl Command for TestCommand {
        fn apply(&self, runtime: &mut BasicWorld) -> Result<(), String> {
            runtime.step_world().map_err(|e| format!("World step failed: {:?}", e))?;
            Ok(())
        }
    }

    #[test]
    fn test_world_runtime_creation() {
        let runtime = WorldRuntime::new();
        assert_eq!(runtime.cursor(), 0);
        assert_eq!(runtime.mode(), &RuntimeMode::Authoritative);
    }

    #[test]
    fn test_world_runtime_set_cursor() {
        let mut runtime = WorldRuntime::new();
        
        runtime.set_cursor(0).expect("Failed to set cursor");
        assert_eq!(runtime.cursor(), 0);
    }

    #[test]
    fn test_world_runtime_set_cursor_invalid() {
        let mut runtime = WorldRuntime::new();
        
        let result = runtime.set_cursor(100);
        assert!(result.is_err());
    }

    #[test]
    fn test_world_runtime_execute_command() {
        let mut runtime = WorldRuntime::new();
        let command = TestCommand;

        runtime.execute_command(&command).expect("Failed to execute command");
        
        assert_eq!(runtime.cursor(), 1);
        assert!(runtime.current_snapshot().is_some());
    }

    #[test]
    fn test_world_runtime_execute_command_observational_mode() {
        let mut runtime = WorldRuntime::new();
        runtime.set_mode(RuntimeMode::Observational);
        let command = TestCommand;

        let result = runtime.execute_command(&command);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Runtime is not authoritative");
    }

    #[test]
    fn test_world_runtime_current_snapshot() {
        let mut runtime = WorldRuntime::new();
        let command = TestCommand;

        runtime.execute_command(&command).expect("Failed to execute command");
        
        let snapshot = runtime.current_snapshot();
        assert!(snapshot.is_some());
        assert_eq!(snapshot.unwrap().tick(), Tick::new(1));
    }

    #[test]
    fn test_world_runtime_state_query() {
        let mut runtime = WorldRuntime::new();
        let command = TestCommand;

        runtime.execute_command(&command).expect("Failed to execute command");
        
        let snapshot = StateQuery::get_at(&runtime, Tick::new(1));
        assert!(snapshot.is_some());
        
        let range = StateQuery::query_range(&runtime, Tick::new(0), Tick::new(1));
        assert_eq!(range.len(), 2);
        
        let latest = StateQuery::latest_snapshot(&runtime);
        assert!(latest.is_some());
    }

    #[test]
    fn test_world_runtime_multiple_commands() {
        let mut runtime = WorldRuntime::new();
        let command = TestCommand;

        for _ in 0..3 {
            runtime.execute_command(&command).expect("Failed to execute command");
        }
        
        assert_eq!(runtime.cursor(), 3);
        
        runtime.set_cursor(2).expect("Failed to set cursor");
        assert_eq!(runtime.cursor(), 2);
        
        runtime.set_cursor(1).expect("Failed to set cursor");
        assert_eq!(runtime.cursor(), 1);
    }

    #[test]
    fn test_world_runtime_default() {
        let runtime: WorldRuntime = Default::default();
        assert_eq!(runtime.cursor(), 0);
        assert_eq!(runtime.mode(), &RuntimeMode::Authoritative);
    }

    #[test]
    fn test_world_runtime_mode_setter() {
        let mut runtime = WorldRuntime::new();
        assert_eq!(runtime.mode(), &RuntimeMode::Authoritative);
        
        runtime.set_mode(RuntimeMode::Observational);
        assert_eq!(runtime.mode(), &RuntimeMode::Observational);
        
        runtime.set_mode(RuntimeMode::Authoritative);
        assert_eq!(runtime.mode(), &RuntimeMode::Authoritative);
    }

    #[test]
    fn test_observation_cursor() {
        let cursor = ObservationCursor(5);
        assert_eq!(cursor.0, 5);
        
        let cursor2 = ObservationCursor(10);
        assert_ne!(cursor, cursor2);
    }
}
