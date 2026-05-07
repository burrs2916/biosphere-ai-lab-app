use biosphere_core::{Perception, Conditions, ConditionSnapshot, ConditionSignal};

/// 基础感知
///
/// [`BasicPerception`] 是 [`Perception`] trait 的基础实现。
///
/// # 设计约束
///
/// - 基础实现：提供默认的感知行为
/// - 可覆盖：应用层可以覆盖此实现
/// - 中立容器：不包含具体的业务逻辑
///
/// # 哲学含义
///
/// BasicPerception 是"基础感知"，而不是"具体感知"。
///
/// 这意味着：
/// - BasicPerception 提供默认的感知行为
/// - 应用层可以覆盖此实现
/// - BasicPerception 不包含具体的业务逻辑
#[derive(Debug, Clone, PartialEq)]
pub struct BasicPerception {
    snapshot: ConditionSnapshot,
}

impl BasicPerception {
    /// 创建新的基础感知
    ///
    /// # 参数
    ///
    /// * `conditions` - 条件
    ///
    /// # 返回值
    ///
    /// 返回新的基础感知
    pub fn new(conditions: &dyn Conditions) -> Self {
        Self {
            snapshot: conditions.snapshot(),
        }
    }

    /// 获取条件快照
    ///
    /// # 返回值
    ///
    /// 返回条件快照的引用
    pub fn snapshot(&self) -> &ConditionSnapshot {
        &self.snapshot
    }
}

impl Perception for BasicPerception {
    type Signal = ConditionSignal;

    /// 获取信号
    ///
    /// # 返回值
    ///
    /// 返回第一个信号，如果没有信号则返回默认信号
    ///
    /// # 设计约束
    ///
    /// - 这是默认实现
    /// - 应用层可以覆盖此方法
    /// - Foundation 层不包含具体的业务逻辑
    fn signal(&self) -> Self::Signal {
        if let Some(signal) = self.snapshot.signals.first() {
            signal.clone()
        } else {
            ConditionSignal {
                kind: "",
                intensity: 0,
            }
        }
    }

    /// 区分信号
    ///
    /// # 参数
    ///
    /// * `a` - 第一个信号
    /// * `b` - 第二个信号
    ///
    /// # 返回值
    ///
    /// 如果两个信号相同，返回 true，否则返回 false
    ///
    /// # 设计约束
    ///
    /// - 这是默认实现
    /// - 应用层可以覆盖此方法
    /// - Foundation 层不包含具体的业务逻辑
    fn distinguish(&self, a: &Self::Signal, b: &Self::Signal) -> bool {
        a.kind == b.kind && a.intensity == b.intensity
    }
}
