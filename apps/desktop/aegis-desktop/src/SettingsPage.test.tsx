import '@testing-library/jest-dom/vitest';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AegisI18nProvider } from '@aegis/ui/i18n';
import { AegisThemeProvider } from '@aegis/ui/theme';
import { SettingsPage } from './SettingsPage';

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear() {
      data.clear();
    },
    getItem(key: string) {
      return data.has(key) ? data.get(key)! : null;
    },
    key(index: number) {
      return Array.from(data.keys())[index] ?? null;
    },
    removeItem(key: string) {
      data.delete(key);
    },
    setItem(key: string, value: string) {
      data.set(key, value);
    },
  } as unknown as Storage;
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.stubGlobal('localStorage', createMemoryStorage());
});

afterEach(() => {
  cleanup();
});

function renderSettings(defaultLocale: 'en' | 'zh-CN' = 'en') {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale={defaultLocale}>
        <SettingsPage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe('SettingsPage', () => {
  it('renders English copy by default', () => {
    renderSettings();

    expect(screen.getByRole('heading', { level: 4 })).toHaveTextContent(
      'Settings',
    );
    expect(screen.getByLabelText(/Theme: Light/i)).toBeInTheDocument();
    expect(screen.getByLabelText('Language')).toHaveTextContent('English');
  });

  it('renders Simplified Chinese copy when the default locale is zh-CN', () => {
    renderSettings('zh-CN');

    expect(screen.getByRole('heading', { level: 4 })).toHaveTextContent('设置');
    expect(screen.getByLabelText(/主题：浅色/i)).toBeInTheDocument();
    expect(screen.getByLabelText('语言')).toHaveTextContent('简体中文');
  });

  it('switches locale, headings, and theme label when the user picks zh-CN', async () => {
    renderSettings('en');

    await userEvent.click(screen.getByLabelText('Language'));
    await userEvent.click(screen.getByRole('option', { name: 'Simplified Chinese' }));

    expect(screen.getByRole('heading', { level: 4 })).toHaveTextContent('设置');
    expect(screen.getByLabelText(/主题：浅色/i)).toBeInTheDocument();
  });
});