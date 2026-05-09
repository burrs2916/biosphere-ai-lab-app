import { get } from 'svelte/store';
import { i18n } from '$lib/i18n';

export interface ErrorAdvice {
  message: string;
  suggestion: string;
  severity: 'error' | 'warning' | 'info';
}

interface ErrorPatternEntry {
  pattern: RegExp;
  messageKey: string;
  suggestionKey: string;
  severity: 'error' | 'warning' | 'info';
}

const errorPatterns: ErrorPatternEntry[] = [
  {
    pattern: /数据集.*不存在|Dataset.*not found|数据集.*已被删除/,
    messageKey: 'errorMessages.datasetNotFound',
    suggestionKey: 'errorMessages.datasetNotFoundSuggestion',
    severity: 'error',
  },
  {
    pattern: /实验.*不存在|Experiment.*not found|实验.*已被删除/,
    messageKey: 'errorMessages.experimentNotFound',
    suggestionKey: 'errorMessages.experimentNotFoundSuggestion',
    severity: 'error',
  },
  {
    pattern: /模型.*不存在|Model.*not found|模型.*已被删除/,
    messageKey: 'errorMessages.modelNotFound',
    suggestionKey: 'errorMessages.modelNotFoundSuggestion',
    severity: 'error',
  },
  {
    pattern: /不支持的数据格式|Data source.*not found|Unknown.*format/,
    messageKey: 'errorMessages.unsupportedFormat',
    suggestionKey: 'errorMessages.unsupportedFormatSuggestion',
    severity: 'error',
  },
  {
    pattern: /数据加载失败|Data.*load.*fail|加载数据/,
    messageKey: 'errorMessages.dataLoadFailed',
    suggestionKey: 'errorMessages.dataLoadFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /数据预览失败|preview.*fail/,
    messageKey: 'errorMessages.previewFailed',
    suggestionKey: 'errorMessages.previewFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /无法读取文件|无法读取CSV|read.*fail|权限/,
    messageKey: 'errorMessages.fileReadFailed',
    suggestionKey: 'errorMessages.fileReadFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /注册.*失败|Register.*fail|重复注册|已存在/,
    messageKey: 'errorMessages.registerFailed',
    suggestionKey: 'errorMessages.registerFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /删除.*失败|Delete.*fail|未被.*引用/,
    messageKey: 'errorMessages.deleteFailed',
    suggestionKey: 'errorMessages.deleteFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /归档.*失败|Archive.*fail|状态.*活跃/,
    messageKey: 'errorMessages.archiveFailed',
    suggestionKey: 'errorMessages.archiveFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /恢复.*失败|Restore.*fail|状态.*归档/,
    messageKey: 'errorMessages.restoreFailed',
    suggestionKey: 'errorMessages.restoreFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /创建.*版本|New.*version|状态.*创建版本/,
    messageKey: 'errorMessages.versionFailed',
    suggestionKey: 'errorMessages.versionFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /训练.*失败|Training.*fail|启动训练/,
    messageKey: 'errorMessages.trainingStartFailed',
    suggestionKey: 'errorMessages.trainingStartFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /停止训练.*失败|Stop.*fail|训练.*结束/,
    messageKey: 'errorMessages.stopFailed',
    suggestionKey: 'errorMessages.stopFailedSuggestion',
    severity: 'warning',
  },
  {
    pattern: /暂停训练.*失败|Pause.*fail|不在运行/,
    messageKey: 'errorMessages.pauseFailed',
    suggestionKey: 'errorMessages.pauseFailedSuggestion',
    severity: 'warning',
  },
  {
    pattern: /恢复训练.*失败|Resume.*fail|不在暂停/,
    messageKey: 'errorMessages.resumeFailed',
    suggestionKey: 'errorMessages.resumeFailedSuggestion',
    severity: 'warning',
  },
  {
    pattern: /推理.*失败|Inference.*fail|无法.*推理/,
    messageKey: 'errorMessages.inferenceFailed',
    suggestionKey: 'errorMessages.inferenceFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /JSON.*格式无效|JSON.*语法|Invalid.*JSON|serde/,
    messageKey: 'errorMessages.jsonFormatError',
    suggestionKey: 'errorMessages.jsonFormatErrorSuggestion',
    severity: 'error',
  },
  {
    pattern: /路径.*无效|invalid.*traversal|遍历序列/,
    messageKey: 'errorMessages.invalidPath',
    suggestionKey: 'errorMessages.invalidPathSuggestion',
    severity: 'error',
  },
  {
    pattern: /网络|连接|connect|network|timeout|超时/,
    messageKey: 'errorMessages.networkError',
    suggestionKey: 'errorMessages.networkErrorSuggestion',
    severity: 'error',
  },
  {
    pattern: /磁盘|disk|空间|写入权限/,
    messageKey: 'errorMessages.diskError',
    suggestionKey: 'errorMessages.diskErrorSuggestion',
    severity: 'error',
  },
  {
    pattern: /数据库|database|DB|sqlite/,
    messageKey: 'errorMessages.databaseError',
    suggestionKey: 'errorMessages.databaseErrorSuggestion',
    severity: 'error',
  },
  {
    pattern: /输入.*为空|empty|cannot be empty/,
    messageKey: 'errorMessages.emptyInput',
    suggestionKey: 'errorMessages.emptyInputSuggestion',
    severity: 'warning',
  },
  {
    pattern: /NaN|无穷|infinite|无效.*值/,
    messageKey: 'errorMessages.invalidValue',
    suggestionKey: 'errorMessages.invalidValueSuggestion',
    severity: 'error',
  },
  {
    pattern: /部署|deploy|正在部署/,
    messageKey: 'errorMessages.deployConflict',
    suggestionKey: 'errorMessages.deployConflictSuggestion',
    severity: 'warning',
  },
  {
    pattern: /未知.*预设|Unknown.*preset|Unknown.*template/,
    messageKey: 'errorMessages.unknownPreset',
    suggestionKey: 'errorMessages.unknownPresetSuggestion',
    severity: 'error',
  },
  {
    pattern: /配置.*无效|Invalid.*config|validate/,
    messageKey: 'errorMessages.configValidation',
    suggestionKey: 'errorMessages.configValidationSuggestion',
    severity: 'error',
  },
  {
    pattern: /保存.*失败|Save.*fail|序列化/,
    messageKey: 'errorMessages.saveFailed',
    suggestionKey: 'errorMessages.saveFailedSuggestion',
    severity: 'error',
  },
  {
    pattern: /加载.*失败|Load.*fail|查询.*失败/,
    messageKey: 'errorMessages.loadFailed',
    suggestionKey: 'errorMessages.loadFailedSuggestion',
    severity: 'error',
  },
];

export function translateError(rawError: string): ErrorAdvice {
  const t = get(i18n.t);
  for (const { pattern, messageKey, suggestionKey, severity } of errorPatterns) {
    if (pattern.test(rawError)) {
      return {
        message: t(messageKey),
        suggestion: t(suggestionKey),
        severity,
      };
    }
  }

  return {
    message: rawError.length > 100 ? rawError.substring(0, 100) + '...' : rawError,
    suggestion: t('errorMessages.defaultSuggestion'),
    severity: 'error',
  };
}

export function formatErrorMessage(rawError: string): string {
  const advice = translateError(rawError);
  return `${advice.message}\n💡 ${advice.suggestion}`;
}
