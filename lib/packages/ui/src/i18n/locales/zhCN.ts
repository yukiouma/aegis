import { en } from './en';

export const zhCN = {
  'language.english': '英语',
  'language.simplifiedChinese': '简体中文',
  'language.current': '当前语言：{name}',

  'app.title': 'Aegis',
  'nav.home': '首页',
  'nav.settings': '设置',
  'home.heading': '首页',
  'home.welcome': '欢迎使用 Aegis。',
  'home.testGreet': '测试问候',
  'settings.heading': '设置',
  'settings.theme.label': '主题：{mode}',
  'settings.theme.dark': '深色',
  'settings.theme.light': '浅色',
  'settings.language.label': '语言',
} satisfies Record<keyof typeof en, string>;