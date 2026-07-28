import { useContext } from 'react';
import { AegisThemeModeContext } from './AegisThemeProvider';

export function useThemeMode() {
  const ctx = useContext(AegisThemeModeContext);
  if (!ctx) {
    throw new Error('useThemeMode must be used inside <AegisThemeProvider>');
  }
  return ctx;
}
