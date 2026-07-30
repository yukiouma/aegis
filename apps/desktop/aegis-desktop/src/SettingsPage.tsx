import type { ChangeEvent } from 'react';
import {
  Box,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Switch,
  Typography,
  type SelectChangeEvent,
} from '@aegis/ui/mui';
import { useI18n, type Locale } from '@aegis/ui/i18n';
import { useThemeMode } from '@aegis/ui/theme';

export function SettingsPage() {
  const { mode, setMode } = useThemeMode();
  const { locale, setLocale, t } = useI18n();

  const handleThemeChange = (event: ChangeEvent<HTMLInputElement>) => {
    setMode(event.target.checked ? 'dark' : 'light');
  };

  const handleLanguageChange = (event: SelectChangeEvent<Locale>) => {
    setLocale(event.target.value as Locale);
  };

  const themeLabel = t('settings.theme.label', {
    mode: t(
      mode === 'dark' ? 'settings.theme.dark' : 'settings.theme.light',
    ),
  });

  return (
    <Box sx={{ p: 4, display: 'flex', flexDirection: 'column', gap: 2 }}>
      <Typography variant="h4" gutterBottom>
        {t('settings.heading')}
      </Typography>
      <FormControlLabel
        control={<Switch checked={mode === 'dark'} onChange={handleThemeChange} />}
        label={themeLabel}
      />
      <FormControl size="small" sx={{ minWidth: 160 }}>
        <InputLabel id="language-label">{t('settings.language.label')}</InputLabel>
        <Select<Locale>
          labelId="language-label"
          value={locale}
          label={t('settings.language.label')}
          onChange={handleLanguageChange}
        >
          <MenuItem value="en">{t('language.english')}</MenuItem>
          <MenuItem value="zh-CN">{t('language.simplifiedChinese')}</MenuItem>
        </Select>
      </FormControl>
    </Box>
  );
}