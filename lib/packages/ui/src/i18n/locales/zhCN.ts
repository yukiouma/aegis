import { en } from './en';

export const zhCN = {
  'language.english': '英语',
  'language.simplifiedChinese': '简体中文',
  'language.current': '当前语言：{name}',
} satisfies Record<keyof typeof en, string>;