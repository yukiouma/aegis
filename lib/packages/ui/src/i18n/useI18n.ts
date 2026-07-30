import { useContext } from 'react';
import { AegisI18nContext } from './AegisI18nProvider';
import type { I18nContextValue } from './types';

export function useI18n(): I18nContextValue {
  const context = useContext(AegisI18nContext);
  if (!context) {
    throw new Error('useI18n must be used inside <AegisI18nProvider>');
  }
  return context;
}