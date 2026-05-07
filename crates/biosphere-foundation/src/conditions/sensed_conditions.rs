use biosphere_core::{Conditions, ConditionSnapshot};
use std::sync::Arc;

/// 感知条件
///
/// [`SensedConditions`] 是 [`Conditions`] 的唯一实现。
///
/// 它描述世界允许被感知的那一部分，是"世界 → 生命"的单向闸门。
///
/// # 设计约束
///
/// - 不可构造：外部代码无法构造 SensedConditions（只能通过 new）
/// - 不可修改：Conditions 是不可变的，是时间值对象
/// - 不可反推：无法从 Conditions 反推世界的完整状态
/// - 单向性：世界 → 生命，生命无法反向影响世界
/// - 中立容器：不包含任何业务逻辑
/// - 时间值对象：每个 Conditions 实例代表特定时间点的观测，不可更新
///
/// # 哲学含义
///
/// SensedConditions 是"世界允许被感知的那一部分"，而不是"世界的完整状态"。
///
/// 这意味着：
/// - UI 无法伪造 Conditions
/// - 生命无法通过 Conditions 修改世界
/// - Conditions 不包含世界的完整信息
/// - Conditions 是"单向闸门"，只允许信息从世界流向生命
/// - SensedConditions 是中立容器，不解释信号含义
/// - Conditions 是时间值对象，代表特定时间点的观测，不可更新
/// - 每个 tick 生成新的 Conditions 实例，而不是更新现有实例
#[derive(Clone, Debug, PartialEq)]
pub struct SensedConditions {
    snapshot: Arc<ConditionSnapshot>,
}

impl SensedConditions {
    /// 创建感知条件
    ///
    /// # 参数
    ///
    /// * `snapshot` - 条件快照
    ///
    /// # 设计约束
    ///
    /// - SensedConditions 不创建信号
    /// - SensedConditions 不解释信号含义
    /// - 信号创建和解释由应用层负责
    pub fn new(snapshot: ConditionSnapshot) -> Self {
        Self {
            snapshot: Arc::new(snapshot),
        }
    }


    /// 返回条件快照
    pub fn snapshot(&self) -> ConditionSnapshot {
        (*self.snapshot).clone()
    }
}

impl Conditions for SensedConditions {
    fn snapshot(&self) -> ConditionSnapshot {
        self.snapshot()
    }
}
