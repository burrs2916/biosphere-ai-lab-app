use crate::perception::entry::PerceptionEntry;
use std::fmt;

/// 感知展开结果
///
/// [`Perception`] 是 Manifest 的线性感知展开结果。
///
/// # 设计约束
///
/// - 线性序列：将树形 Manifest 展开为线性序列
/// - 感知顺序：按照人类感知的顺序排列
/// - 时间绑定：每个 Perception 都与特定时间点绑定
/// - 不可变：一旦创建就不可修改
///
/// # 哲学含义
///
/// Perception 是"人类如何一步一步'看'Manifest 的结果"，而不是"渲染结果"。
///
/// 这意味着：
/// - Perception 是感知顺序，不是渲染顺序
/// - Perception 是人类可读的，不是机器可读的
/// - Perception 是 UI 的输入，不是 UI 的输出
#[derive(Debug, Clone, PartialEq)]
pub struct Perception {
    /// 时间点
    pub time: u64,
    /// 感知条目
    pub entries: Vec<PerceptionEntry>,
}

impl Perception {
    /// 创建新的感知
    ///
    /// # 参数
    ///
    /// * `time` - 时间点
    /// * `entries` - 感知条目
    ///
    /// # 示例
    ///
    /// ```rust
    /// use biosphere_foundation::perception::{Perception, PerceptionEntry};
    ///
    /// let perception = Perception::new(42, vec![]);
    /// ```
    pub fn new(time: u64, entries: Vec<PerceptionEntry>) -> Self {
        Self { time, entries }
    }

    /// 获取时间点
    ///
    /// # 返回值
    ///
    /// 返回时间点
    pub fn time(&self) -> u64 {
        self.time
    }

    /// 获取感知条目
    ///
    /// # 返回值
    ///
    /// 返回感知条目的引用
    pub fn entries(&self) -> &[PerceptionEntry] {
        &self.entries
    }
}

impl fmt::Display for Perception {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Perception @ time {}", self.time)?;
        for entry in &self.entries {
            writeln!(f, "  {}", entry)?;
        }
        Ok(())
    }
}