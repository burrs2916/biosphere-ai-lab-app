interface ErrorPattern {
  pattern: RegExp;
  title: string;
  message: string;
  suggestion: string;
}

const errorPatterns: ErrorPattern[] = [
  {
    pattern: /ENOENT|no such file|not found|文件不存在/i,
    title: '文件未找到',
    message: '指定的文件或路径不存在',
    suggestion: '请检查文件路径是否正确，确保文件未被移动或删除',
  },
  {
    pattern: /EACCES|permission denied|权限/i,
    title: '权限不足',
    message: '没有足够的权限访问该资源',
    suggestion: '请检查文件权限，或使用具有足够权限的账户操作',
  },
  {
    pattern: /ENOSPC|no space left|磁盘空间/i,
    title: '磁盘空间不足',
    message: '存储空间已满，无法完成操作',
    suggestion: '请清理磁盘空间后重试',
  },
  {
    pattern: /ECONNREFUSED|connection refused|连接被拒绝/i,
    title: '连接失败',
    message: '无法连接到目标服务',
    suggestion: '请检查网络连接，确认目标服务是否正在运行',
  },
  {
    pattern: /ETIMEDOUT|timeout|超时/i,
    title: '操作超时',
    message: '操作耗时过长，已自动取消',
    suggestion: '请检查网络连接，或稍后重试。如果数据集较大，可能需要更长时间',
  },
  {
    pattern: /invalid format|格式错误|parse error|解析失败/i,
    title: '数据格式错误',
    message: '数据格式不符合预期',
    suggestion: '请检查数据文件格式是否正确，确保与声明的格式一致',
  },
  {
    pattern: /duplicate|重复|already exist|已存在/i,
    title: '数据重复',
    message: '检测到重复的数据或资源',
    suggestion: '该名称可能已被使用，请尝试使用不同的名称',
  },
  {
    pattern: /schema mismatch|结构不匹配|column.*mismatch/i,
    title: '数据结构不匹配',
    message: '数据列结构发生了变化',
    suggestion: '数据可能已被外部修改，请重新注册数据集以更新结构信息',
  },
  {
    pattern: /out of memory|内存不足|OOM/i,
    title: '内存不足',
    message: '操作需要更多内存才能完成',
    suggestion: '请尝试处理较小的数据子集，或关闭其他应用释放内存',
  },
  {
    pattern: /checksum|digest|校验和不匹配/i,
    title: '数据校验失败',
    message: '数据完整性校验未通过，文件可能已损坏',
    suggestion: '文件可能在传输过程中损坏，请重新下载或复制数据文件',
  },
  {
    pattern: /version conflict|版本冲突/i,
    title: '版本冲突',
    message: '数据版本存在冲突',
    suggestion: '数据可能被其他进程修改，请刷新后重试',
  },
  {
    pattern: /Not implemented|未实现/i,
    title: '功能暂未实现',
    message: '该功能正在开发中，暂不可用',
    suggestion: '请关注后续版本更新',
  },
  {
    pattern: /network|网络/i,
    title: '网络错误',
    message: '网络连接出现问题',
    suggestion: '请检查网络设置，确保可以访问所需服务',
  },
  {
    pattern: /serialization|序列化|deserialization|反序列化/i,
    title: '数据处理错误',
    message: '数据在处理过程中出现格式转换错误',
    suggestion: '请检查数据是否包含特殊字符或不支持的格式',
  },
];

export interface LocalizedError {
  title: string;
  message: string;
  suggestion: string;
  originalError: string;
}

export function localizeError(error: string | Error | unknown): LocalizedError {
  const errorStr = error instanceof Error ? error.message : String(error || '未知错误');

  for (const { pattern, title, message, suggestion } of errorPatterns) {
    if (pattern.test(errorStr)) {
      return { title, message, suggestion, originalError: errorStr };
    }
  }

  return {
    title: '操作失败',
    message: errorStr.length > 100 ? errorStr.substring(0, 100) + '...' : errorStr,
    suggestion: '请稍后重试，如果问题持续存在，请查看详细错误信息',
    originalError: errorStr,
  };
}

export function formatLocalizedError(error: string | Error | unknown): string {
  const localized = localizeError(error);
  let result = localized.message;
  if (localized.suggestion) {
    result += `\n💡 ${localized.suggestion}`;
  }
  return result;
}
