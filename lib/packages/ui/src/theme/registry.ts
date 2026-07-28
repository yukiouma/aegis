import type { Theme } from '@mui/material/styles';
import { lightTheme } from './themes/light';
import { darkTheme } from './themes/dark';
import type { ThemeMode } from './types';

const themes: Record<ThemeMode, Theme> = {
  light: lightTheme,
  dark: darkTheme,
};

export function getTheme(mode: ThemeMode): Theme {
  return themes[mode];
}
