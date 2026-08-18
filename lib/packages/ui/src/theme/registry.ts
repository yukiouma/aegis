import type { Theme } from '@mui/material/styles';
import { lightTheme } from './themes/light';
import { darkTheme } from './themes/dark';
import {
  anyaTheme,
  chihiroTheme,
  ntdTheme,
  siblyTheme,
  totoroTheme,
  xiTheme,
} from './themes';
import type { ThemeMode } from './types';

const themes: Record<ThemeMode, Theme> = {
  light: lightTheme,
  dark: darkTheme,
  anya: anyaTheme,
  chihiro: chihiroTheme,
  ntd: ntdTheme,
  sibly: siblyTheme,
  totoro: totoroTheme,
  xi: xiTheme,
};

export function getTheme(mode: ThemeMode): Theme {
  return themes[mode];
}