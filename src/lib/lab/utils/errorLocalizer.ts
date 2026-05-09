import { get } from 'svelte/store';
import { i18n } from '$lib/i18n';

interface ErrorPattern {
  pattern: RegExp;
  titleKey: string;
  messageKey: string;
  suggestionKey: string;
}

const errorPatterns: ErrorPattern[] = [
  {
    pattern: /ENOENT|no such file|not found|文件不存在/i,
    titleKey: 'errorLocalizer.fileNotFound',
    messageKey: 'errorLocalizer.fileNotFoundMsg',
    suggestionKey: 'errorLocalizer.fileNotFoundSuggestion',
  },
  {
    pattern: /EACCES|permission denied|权限/i,
    titleKey: 'errorLocalizer.permissionDenied',
    messageKey: 'errorLocalizer.permissionDeniedMsg',
    suggestionKey: 'errorLocalizer.permissionDeniedSuggestion',
  },
  {
    pattern: /ENOSPC|no space left|磁盘空间/i,
    titleKey: 'errorLocalizer.diskFull',
    messageKey: 'errorLocalizer.diskFullMsg',
    suggestionKey: 'errorLocalizer.diskFullSuggestion',
  },
  {
    pattern: /ECONNREFUSED|connection refused|连接被拒绝/i,
    titleKey: 'errorLocalizer.connectionFailed',
    messageKey: 'errorLocalizer.connectionFailedMsg',
    suggestionKey: 'errorLocalizer.connectionFailedSuggestion',
  },
  {
    pattern: /ETIMEDOUT|timeout|超时/i,
    titleKey: 'errorLocalizer.timeout',
    messageKey: 'errorLocalizer.timeoutMsg',
    suggestionKey: 'errorLocalizer.timeoutSuggestion',
  },
  {
    pattern: /invalid format|格式错误|parse error|解析失败/i,
    titleKey: 'errorLocalizer.formatError',
    messageKey: 'errorLocalizer.formatErrorMsg',
    suggestionKey: 'errorLocalizer.formatErrorSuggestion',
  },
  {
    pattern: /duplicate|重复|already exist|已存在/i,
    titleKey: 'errorLocalizer.duplicate',
    messageKey: 'errorLocalizer.duplicateMsg',
    suggestionKey: 'errorLocalizer.duplicateSuggestion',
  },
  {
    pattern: /schema mismatch|结构不匹配|column.*mismatch/i,
    titleKey: 'errorLocalizer.schemaMismatch',
    messageKey: 'errorLocalizer.schemaMismatchMsg',
    suggestionKey: 'errorLocalizer.schemaMismatchSuggestion',
  },
  {
    pattern: /out of memory|内存不足|OOM/i,
    titleKey: 'errorLocalizer.outOfMemory',
    messageKey: 'errorLocalizer.outOfMemoryMsg',
    suggestionKey: 'errorLocalizer.outOfMemorySuggestion',
  },
  {
    pattern: /checksum|digest|校验和不匹配/i,
    titleKey: 'errorLocalizer.checksumFailed',
    messageKey: 'errorLocalizer.checksumFailedMsg',
    suggestionKey: 'errorLocalizer.checksumFailedSuggestion',
  },
  {
    pattern: /version conflict|版本冲突/i,
    titleKey: 'errorLocalizer.versionConflict',
    messageKey: 'errorLocalizer.versionConflictMsg',
    suggestionKey: 'errorLocalizer.versionConflictSuggestion',
  },
  {
    pattern: /Not implemented|未实现/i,
    titleKey: 'errorLocalizer.notImplemented',
    messageKey: 'errorLocalizer.notImplementedMsg',
    suggestionKey: 'errorLocalizer.notImplementedSuggestion',
  },
  {
    pattern: /network|网络/i,
    titleKey: 'errorLocalizer.networkError',
    messageKey: 'errorLocalizer.networkErrorMsg',
    suggestionKey: 'errorLocalizer.networkErrorSuggestion',
  },
  {
    pattern: /serialization|序列化|deserialization|反序列化/i,
    titleKey: 'errorLocalizer.serializationError',
    messageKey: 'errorLocalizer.serializationErrorMsg',
    suggestionKey: 'errorLocalizer.serializationErrorSuggestion',
  },
];

export interface LocalizedError {
  title: string;
  message: string;
  suggestion: string;
  originalError: string;
}

export function localizeError(error: string | Error | unknown): LocalizedError {
  const t = get(i18n.t);
  const errorStr = error instanceof Error ? error.message : String(error || t('errorLocalizer.unknownError'));

  for (const { pattern, titleKey, messageKey, suggestionKey } of errorPatterns) {
    if (pattern.test(errorStr)) {
      return {
        title: t(titleKey),
        message: t(messageKey),
        suggestion: t(suggestionKey),
        originalError: errorStr,
      };
    }
  }

  return {
    title: t('errorLocalizer.operationFailed'),
    message: errorStr.length > 100 ? errorStr.substring(0, 100) + '...' : errorStr,
    suggestion: t('errorLocalizer.operationFailedSuggestion'),
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
