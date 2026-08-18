import { describe, it, expect } from 'vitest';
import { getTheme } from './registry';
import { lightTheme } from './themes/light';
import { darkTheme } from './themes/dark';
import { anyaTheme } from './themes/anyaTheme';
import { chihiroTheme } from './themes/chihiroTheme';
import { ntdTheme } from './themes/ntdTheme';
import { siblyTheme } from './themes/siblyTheme';
import { totoroTheme } from './themes/totoroTheme';
import { xiTheme } from './themes/xiTheme';

describe('theme registry', () => {
  it('light mode returns the light theme', () => {
    expect(getTheme('light')).toBe(lightTheme);
  });

  it('dark mode returns the dark theme', () => {
    expect(getTheme('dark')).toBe(darkTheme);
  });

  it('anya returns the anya theme (light palette)', () => {
    expect(getTheme('anya')).toBe(anyaTheme);
    expect(getTheme('anya').palette.mode).toBe('light');
  });

  it('chihiro returns the chihiro theme (light palette)', () => {
    expect(getTheme('chihiro')).toBe(chihiroTheme);
    expect(getTheme('chihiro').palette.mode).toBe('light');
  });

  it('ntd returns the ntd theme (dark palette)', () => {
    expect(getTheme('ntd')).toBe(ntdTheme);
    expect(getTheme('ntd').palette.mode).toBe('dark');
  });

  it('sibly returns the sibly theme (dark palette)', () => {
    expect(getTheme('sibly')).toBe(siblyTheme);
    expect(getTheme('sibly').palette.mode).toBe('dark');
  });

  it('totoro returns the totoro theme (palette.mode defaults to light)', () => {
    expect(getTheme('totoro')).toBe(totoroTheme);
    expect(getTheme('totoro').palette.mode).toBe('light');
  });

  it('xi returns the xi theme (dark palette)', () => {
    expect(getTheme('xi')).toBe(xiTheme);
    expect(getTheme('xi').palette.mode).toBe('dark');
  });
});