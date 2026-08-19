import { createContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import { ThemeProvider } from '@mui/material/styles';
import CssBaseline from '@mui/material/CssBaseline';
import type { ThemeMode } from './types';
import { getTheme } from './registry';

const STORAGE_KEY = 'aegis:theme:mode';

function isThemeMode(value: string | null): value is ThemeMode {
  return (
    value === 'light' ||
    value === 'dark' ||
    value === 'anya' ||
    value === 'chihiro' ||
    value === 'ntd' ||
    value === 'sibly' ||
    value === 'totoro' ||
    value === 'xi'
  );
}

function readInitialMode(): ThemeMode {
  if (typeof window === 'undefined') return 'light';
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (isThemeMode(stored)) return stored;
  } catch {
    // localStorage may throw in private modes / sandboxed contexts.
  }
  return 'light';
}

export interface AegisThemeModeContextValue {
  mode: ThemeMode;
  setMode: (mode: ThemeMode) => void;
}

export const AegisThemeModeContext = createContext<AegisThemeModeContextValue | null>(null);

export interface AegisThemeProviderProps {
  children: ReactNode;
  onModeChange?: (mode: ThemeMode) => void;
}

export function AegisThemeProvider({ children, onModeChange }: AegisThemeProviderProps) {
  const [mode, setModeState] = useState<ThemeMode>(readInitialMode);

  const setMode = useCallback((next: ThemeMode) => {
    setModeState(next);
  }, []);

  useEffect(() => {
    if (typeof window !== 'undefined') {
      try {
        window.localStorage.setItem(STORAGE_KEY, mode);
      } catch {
        // ignore write failures (quota, private mode)
      }
    }
    onModeChange?.(mode);
  }, [mode, onModeChange]);

  return (
    <AegisThemeModeContext.Provider value={{ mode, setMode }}>
      <ThemeProvider theme={getTheme(mode)}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </AegisThemeModeContext.Provider>
  );
}
