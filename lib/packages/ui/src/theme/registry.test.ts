import { describe, it, expect } from 'vitest';
import { getTheme } from './registry';
import { lightTheme } from './themes/light';
import { darkTheme } from './themes/dark';

describe('theme registry', () => {
  it('light mode returns the light theme', () => {
    expect(getTheme('light')).toBe(lightTheme);
  });

  it('dark mode returns the dark theme', () => {
    expect(getTheme('dark')).toBe(darkTheme);
  });

  it('returned light theme has palette.mode === "light"', () => {
    expect(getTheme('light').palette.mode).toBe('light');
  });

  it('returned dark theme has palette.mode === "dark"', () => {
    expect(getTheme('dark').palette.mode).toBe('dark');
  });
});
