use biosphere_core::{Representation, Perception};
use crate::ontology::basic_perception::BasicPerception;

/// 基础表征
///
/// [`BasicRepresentation`] 是 [`Representation`] trait 的基础实现。
///
/// # 设计约束
///
/// - 基础实现：提供默认的表征行为
/// - 可覆盖：应用层可以覆盖此实现
/// - 中立容器：不包含具体的业务逻辑
///
/// # 哲学含义
///
/// BasicRepresentation 是"基础表征"，而不是"具体表征"。
///
/// 这意味着：
/// - BasicRepresentation 提供默认的表征行为
/// - 应用层可以覆盖此实现
/// - BasicRepresentation 不包含具体的业务逻辑
#[derive(Debug, Clone, PartialEq)]
pub struct BasicRepresentation {
    perception: BasicPerception,
}

impl BasicRepresentation {
    /// 创建新的基础表征
    ///
    /// # 参数
    ///
    /// * `perception` - 感知
    ///
    /// # 返回值
    ///
    /// 返回新的基础表征
    pub fn new(perception: BasicPerception) -> Self {
        Self {
            perception,
        }
    }

    /// 获取感知
    ///
    /// # 返回值
    ///
    /// 返回感知的引用
    pub fn perception(&self) -> &BasicPerception {
        &self.perception
    }
}

impl Representation for BasicRepresentation {
    type Dimension = BasicRepresentationDimension;
    type Topology = BasicRepresentationTopology;
    type Data = BasicRepresentationData;

    /// 获取维度
    ///
    /// # 返回值
    ///
    /// 返回表征的维度集合
    ///
    /// # 设计约束
    ///
    /// - 这是占位实现，返回空切片
    /// - 应用层必须覆盖此方法以提供实际的维度
    /// - Foundation 层不包含具体的业务逻辑
    ///
    /// # 注意
    ///
    /// 此默认实现仅用于编译通过，实际使用时应用层应该提供有意义的实现。
    fn dimensions(&self) -> &[Self::Dimension] {
        &[]
    }

    /// 获取拓扑
    ///
    /// # 返回值
    ///
    /// 返回表征的拓扑结构
    ///
    /// # 设计约束
    ///
    /// - 这是占位实现，返回 None
    /// - 应用层必须覆盖此方法以提供实际的拓扑
    /// - Foundation 层不包含具体的业务逻辑
    ///
    /// # 注意
    ///
    /// 此默认实现仅用于编译通过，实际使用时应用层应该提供有意义的实现。
    fn topology(&self) -> Self::Topology {
        BasicRepresentationTopology::None
    }

    /// 获取数据
    ///
    /// # 返回值
    ///
    /// 返回表征的原始数据
    ///
    /// # 设计约束
    ///
    /// - 这是默认实现
    /// - 应用层可以覆盖此方法
    /// - Foundation 层不包含具体的业务逻辑
    fn data(&self) -> Self::Data {
        BasicRepresentationData::new(self.perception.signal())
    }
}

/// 基础表征维度
///
/// [`BasicRepresentationDimension`] 定义了基础表征的维度类型。
///
/// # 设计约束
///
/// - 基础实现：提供默认的维度类型
/// - 可扩展：应用层可以定义自己的维度类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicRepresentationDimension {
    /// 无维度
    None,
}

/// 基础表征拓扑
///
/// [`BasicRepresentationTopology`] 定义了基础表征的拓扑类型。
///
/// # 设计约束
///
/// - 基础实现：提供默认的拓扑类型
/// - 可扩展：应用层可以定义自己的拓扑类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicRepresentationTopology {
    /// 无拓扑
    None,
}

/// 基础表征数据
///
/// [`BasicRepresentationData`] 定义了基础表征的数据类型。
///
/// # 设计约束
///
/// - 基础实现：提供默认的数据类型
/// - 可扩展：应用层可以定义自己的数据类型
#[derive(Debug, Clone)]
pub struct BasicRepresentationData {
    signal: biosphere_core::ConditionSignal,
}

impl BasicRepresentationData {
    pub fn new(signal: biosphere_core::ConditionSignal) -> Self {
        Self { signal }
    }

    pub fn signal(&self) -> &biosphere_core::ConditionSignal {
        &self.signal
    }
}
