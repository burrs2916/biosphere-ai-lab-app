export interface ErrorAdvice {
  message: string;
  suggestion: string;
  severity: 'error' | 'warning' | 'info';
}

const errorPatterns: { pattern: RegExp; advice: ErrorAdvice }[] = [
  {
    pattern: /数据集.*不存在|Dataset.*not found|数据集.*已被删除/,
    advice: {
      message: '数据集不存在',
      suggestion: '该数据集可能已被删除或ID不正确。请刷新数据集列表，确认数据集是否仍然存在。',
      severity: 'error',
    },
  },
  {
    pattern: /实验.*不存在|Experiment.*not found|实验.*已被删除/,
    advice: {
      message: '实验不存在',
      suggestion: '该实验可能已被删除或ID不正确。请刷新实验列表，确认实验是否仍然存在。',
      severity: 'error',
    },
  },
  {
    pattern: /模型.*不存在|Model.*not found|模型.*已被删除/,
    advice: {
      message: '模型不存在',
      suggestion: '该模型可能已被删除或ID不正确。请刷新模型列表，确认模型是否仍然存在。',
      severity: 'error',
    },
  },
  {
    pattern: /不支持的数据格式|Data source.*not found|Unknown.*format/,
    advice: {
      message: '不支持的数据格式',
      suggestion: '请使用支持的数据格式：CSV、JSON、Parquet、Excel、Text、Image、Binary、TFRecord、HuggingFace、Database。',
      severity: 'error',
    },
  },
  {
    pattern: /数据加载失败|Data.*load.*fail|加载数据/,
    advice: {
      message: '数据加载失败',
      suggestion: '请检查文件路径是否正确、文件格式是否匹配、文件是否损坏。对于CSV文件，请确认编码为UTF-8。',
      severity: 'error',
    },
  },
  {
    pattern: /数据预览失败|preview.*fail/,
    advice: {
      message: '数据预览失败',
      suggestion: '请检查文件是否存在、偏移量和限制参数是否合理。尝试减少预览行数或检查文件编码。',
      severity: 'error',
    },
  },
  {
    pattern: /无法读取文件|无法读取CSV|read.*fail|权限/,
    advice: {
      message: '文件读取失败',
      suggestion: '请检查文件路径是否正确、文件是否存在、是否有读取权限。对于CSV文件，请确认文件编码为UTF-8。',
      severity: 'error',
    },
  },
  {
    pattern: /注册.*失败|Register.*fail|重复注册|已存在/,
    advice: {
      message: '数据集注册失败',
      suggestion: '数据集可能已存在（重复注册）。请检查数据集列表中是否已有同名数据集，或使用不同的名称重新注册。',
      severity: 'error',
    },
  },
  {
    pattern: /删除.*失败|Delete.*fail|未被.*引用/,
    advice: {
      message: '删除操作失败',
      suggestion: '请确认该数据集未被任何实验引用。如果数据集正在被使用，请先取消关联的实验后再删除。',
      severity: 'error',
    },
  },
  {
    pattern: /归档.*失败|Archive.*fail|状态.*活跃/,
    advice: {
      message: '归档操作失败',
      suggestion: '请确认数据集当前状态为"活跃"。只有活跃状态的数据集才能被归档。',
      severity: 'error',
    },
  },
  {
    pattern: /恢复.*失败|Restore.*fail|状态.*归档/,
    advice: {
      message: '恢复操作失败',
      suggestion: '请确认数据集当前状态为"已归档"。只有已归档的数据集才能被恢复。',
      severity: 'error',
    },
  },
  {
    pattern: /创建.*版本|New.*version|状态.*创建版本/,
    advice: {
      message: '创建版本失败',
      suggestion: '只有活跃状态的数据集才能创建新版本。请先恢复数据集，或检查数据文件是否仍然存在。',
      severity: 'error',
    },
  },
  {
    pattern: /训练.*失败|Training.*fail|启动训练/,
    advice: {
      message: '训练启动失败',
      suggestion: '请检查数据路径是否正确、配置文件是否完整、引擎是否可用。确认所有必填字段已填写。',
      severity: 'error',
    },
  },
  {
    pattern: /停止训练.*失败|Stop.*fail|训练.*结束/,
    advice: {
      message: '停止训练失败',
      suggestion: '训练可能已经结束或不在运行状态。请刷新实验列表查看最新状态。',
      severity: 'warning',
    },
  },
  {
    pattern: /暂停训练.*失败|Pause.*fail|不在运行/,
    advice: {
      message: '暂停训练失败',
      suggestion: '训练可能不在运行中。请确认实验状态为"训练中"。',
      severity: 'warning',
    },
  },
  {
    pattern: /恢复训练.*失败|Resume.*fail|不在暂停/,
    advice: {
      message: '恢复训练失败',
      suggestion: '训练可能不在暂停状态。请确认实验状态为"已暂停"。',
      severity: 'warning',
    },
  },
  {
    pattern: /推理.*失败|Inference.*fail|无法.*推理/,
    advice: {
      message: '推理执行失败',
      suggestion: '请确认实验已完成训练（状态为"已完成"）。训练未完成的实验无法进行推理。',
      severity: 'error',
    },
  },
  {
    pattern: /JSON.*格式无效|JSON.*语法|Invalid.*JSON|serde/,
    advice: {
      message: 'JSON格式错误',
      suggestion: '请检查JSON语法是否正确，所有必填字段是否完整。可以使用JSON验证工具检查格式。',
      severity: 'error',
    },
  },
  {
    pattern: /路径.*无效|invalid.*traversal|遍历序列/,
    advice: {
      message: '路径包含无效字符',
      suggestion: '请使用绝对路径，路径中不能包含 ".." 或 "~" 等特殊字符。请提供完整的文件路径。',
      severity: 'error',
    },
  },
  {
    pattern: /网络|连接|connect|network|timeout|超时/,
    advice: {
      message: '网络连接异常',
      suggestion: '请检查网络连接是否正常。如果使用HuggingFace Hub，请确认网络可以访问huggingface.co。',
      severity: 'error',
    },
  },
  {
    pattern: /磁盘|disk|空间|写入权限/,
    advice: {
      message: '磁盘操作异常',
      suggestion: '请检查磁盘空间是否充足、是否有写入权限。尝试清理不需要的文件释放空间。',
      severity: 'error',
    },
  },
  {
    pattern: /数据库|database|DB|sqlite/,
    advice: {
      message: '数据库异常',
      suggestion: '数据库操作失败。请稍后重试，如持续失败请检查数据库文件是否损坏。可以尝试重启应用。',
      severity: 'error',
    },
  },
  {
    pattern: /输入.*为空|empty|cannot be empty/,
    advice: {
      message: '输入不能为空',
      suggestion: '请填写所有必填字段。检查名称、路径等关键字段是否已填写。',
      severity: 'warning',
    },
  },
  {
    pattern: /NaN|无穷|infinite|无效.*值/,
    advice: {
      message: '数据包含无效值',
      suggestion: '输入数据包含NaN或无穷值。请检查数据质量，对数据进行清洗后再使用。',
      severity: 'error',
    },
  },
  {
    pattern: /部署|deploy|正在部署/,
    advice: {
      message: '模型部署冲突',
      suggestion: '该模型当前正在部署中。请先取消部署后再执行此操作。',
      severity: 'warning',
    },
  },
  {
    pattern: /未知.*预设|Unknown.*preset|Unknown.*template/,
    advice: {
      message: '未知的预设类型',
      suggestion: '请使用支持的预设类型。可用的预设包括：llm_pretraining、sft、rlhf。',
      severity: 'error',
    },
  },
  {
    pattern: /配置.*无效|Invalid.*config|validate/,
    advice: {
      message: '配置验证失败',
      suggestion: '请检查所有配置参数是否正确。确认必填字段已填写，数值范围合理。',
      severity: 'error',
    },
  },
  {
    pattern: /保存.*失败|Save.*fail|序列化/,
    advice: {
      message: '保存操作失败',
      suggestion: '请检查磁盘空间是否充足、是否有写入权限。尝试更换保存路径。',
      severity: 'error',
    },
  },
  {
    pattern: /加载.*失败|Load.*fail|查询.*失败/,
    advice: {
      message: '数据加载失败',
      suggestion: '请稍后重试。如持续失败，请检查数据库状态或重启应用。',
      severity: 'error',
    },
  },
];

export function translateError(rawError: string): ErrorAdvice {
  for (const { pattern, advice } of errorPatterns) {
    if (pattern.test(rawError)) {
      return advice;
    }
  }

  return {
    message: rawError.length > 100 ? rawError.substring(0, 100) + '...' : rawError,
    suggestion: '操作未能完成。请稍后重试，如持续失败请联系技术支持。',
    severity: 'error',
  };
}

export function formatErrorMessage(rawError: string): string {
  const advice = translateError(rawError);
  return `${advice.message}\n💡 ${advice.suggestion}`;
}
