import { writable, derived, get } from 'svelte/store';
import zhCN from './locales/zh-CN';
import en from './locales/en';

export type Locale = 'zh-CN' | 'en';

const translations: Record<Locale, any> = {
  'zh-CN': zhCN,
  en: en,
};

const STORAGE_KEY = 'biosphere-ai-lab-locale';

function getDefaultLocale(): Locale {
  if (typeof window !== 'undefined') {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === 'zh-CN' || stored === 'en') {
      return stored;
    }
    const browserLang = navigator.language;
    if (browserLang.startsWith('zh')) {
      return 'zh-CN';
    }
  }
  return 'zh-CN';
}

function createI18n() {
  const { subscribe, set } = writable<Locale>(getDefaultLocale());

  return {
    subscribe,

    setLocale(locale: Locale) {
      set(locale);
      if (typeof window !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, locale);
      }
    },

    getLocale(): Locale {
      return get({ subscribe });
    },

    t: derived({ subscribe }, ($locale) => {
      return (key: string, params?: Record<string, string | number>): string => {
        const keys = key.split('.');
        let value: any = translations[$locale];

        for (const k of keys) {
          if (value && typeof value === 'object' && k in value) {
            value = value[k];
          } else {
            return key;
          }
        }

        if (typeof value !== 'string') {
          return key;
        }

        if (params) {
          return value.replace(/\{(\w+)\}/g, (_, paramKey) => {
            return String(params[paramKey] ?? `{${paramKey}}`);
          });
        }

        return value;
      };
    }),

    translate: derived({ subscribe }, ($locale) => {
      return translations[$locale];
    }),
  };
}

export const i18n = createI18n();
export const t = i18n.t;
export const translate = i18n.translate;

export function formatTimeAgo(dateStr: string, locale: Locale = 'zh-CN'): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diff = now - then;

  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (locale === 'zh-CN') {
    if (diff < 60000) return '刚刚';
    if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} 小时前`;
    return `${Math.floor(diff / 86400000)} 天前`;
  } else {
    if (diff < 60000) return 'just now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)} minutes ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} hours ago`;
    return `${Math.floor(diff / 86400000)} days ago`;
  }
}

export function formatUptime(seconds: number, locale: Locale = 'zh-CN'): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}m ${s}s`;
  }
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  return `${h}h ${m}m`;
}

export function formatLastRefresh(seconds: number, locale: Locale = 'zh-CN'): string {
  if (locale === 'zh-CN') {
    if (seconds < 5) return '刚刚刷新';
    if (seconds < 60) return `${seconds}秒前`;
    return `${Math.floor(seconds / 60)}分钟前`;
  } else {
    if (seconds < 5) return 'just refreshed';
    if (seconds < 60) return `${seconds} seconds ago`;
    return `${Math.floor(seconds / 60)} minutes ago`;
  }
}
