import { useEffect } from 'react';
import { useI18n } from '@aegis/ui/i18n';

export function DocumentLangSync(): null {
  const { locale } = useI18n();
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);
  return null;
}