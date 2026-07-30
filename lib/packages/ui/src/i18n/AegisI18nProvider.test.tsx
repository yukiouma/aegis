import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AegisI18nProvider } from './AegisI18nProvider';
import { useI18n } from './useI18n';

const STORAGE_KEY = 'aegis:i18n:locale';

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

function ReadI18n() {
  const { locale, t } = useI18n();
  return (
    <>
      <span data-testid="locale">{locale}</span>
      <span data-testid="language-name">
        {t('language.simplifiedChinese')}
      </span>
      <span data-testid="number-interpolation">
        {t('language.current', { name: 42 })}
      </span>
      <span data-testid="missing-interpolation">
        {t('language.current')}
      </span>
    </>
  );
}

function SetZhCN() {
  const { setLocale } = useI18n();
  return <button onClick={() => setLocale('zh-CN')}>set-zh-CN</button>;
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.stubGlobal('localStorage', createMemoryStorage());
});

describe('AegisI18nProvider', () => {
  it('defaults to English and writes the resolved locale on mount', () => {
    render(
      <AegisI18nProvider>
        <ReadI18n />
      </AegisI18nProvider>,
    );

    expect(screen.getByTestId('locale')).toHaveTextContent('en');
    expect(screen.getByTestId('language-name')).toHaveTextContent(
      'Simplified Chinese',
    );
    expect(localStorage.getItem(STORAGE_KEY)).toBe('en');
  });

  it('uses an explicit default locale when storage has no valid locale', () => {
    render(
      <AegisI18nProvider defaultLocale="zh-CN">
        <ReadI18n />
      </AegisI18nProvider>,
    );

    expect(screen.getByTestId('locale')).toHaveTextContent('zh-CN');
    expect(screen.getByTestId('language-name')).toHaveTextContent('简体中文');
  });

  it('uses defaultLocale only during initialization', () => {
    const { rerender } = render(
      <AegisI18nProvider defaultLocale="en">
        <ReadI18n />
      </AegisI18nProvider>,
    );

    rerender(
      <AegisI18nProvider defaultLocale="zh-CN">
        <ReadI18n />
      </AegisI18nProvider>,
    );

    expect(screen.getByTestId('locale')).toHaveTextContent('en');
  });

  it('prefers a valid stored locale over the explicit default', () => {
    localStorage.setItem(STORAGE_KEY, 'zh-CN');

    render(
      <AegisI18nProvider defaultLocale="en">
        <ReadI18n />
      </AegisI18nProvider>,
    );

    expect(screen.getByTestId('locale')).toHaveTextContent('zh-CN');
  });

  it('ignores an invalid stored locale and uses the explicit default', () => {
    localStorage.setItem(STORAGE_KEY, 'fr');

    render(
      <AegisI18nProvider defaultLocale="zh-CN">
        <ReadI18n />
      </AegisI18nProvider>,
    );

    expect(screen.getByTestId('locale')).toHaveTextContent('zh-CN');
    expect(localStorage.getItem(STORAGE_KEY)).toBe('zh-CN');
  });

  it('updates translations and persistence when setLocale is called', async () => {
    render(
      <AegisI18nProvider>
        <ReadI18n />
        <SetZhCN />
      </AegisI18nProvider>,
    );

    await userEvent.click(screen.getByText('set-zh-CN'));

    expect(screen.getByTestId('locale')).toHaveTextContent('zh-CN');
    expect(screen.getByTestId('language-name')).toHaveTextContent('简体中文');
    expect(localStorage.getItem(STORAGE_KEY)).toBe('zh-CN');
  });

  it('interpolates values and preserves missing placeholders', () => {
    render(
      <AegisI18nProvider>
        <ReadI18n />
      </AegisI18nProvider>,
    );

    expect(screen.getByTestId('number-interpolation')).toHaveTextContent(
      'Language: 42',
    );
    expect(screen.getByTestId('missing-interpolation')).toHaveTextContent(
      'Language: {name}',
    );
  });

  it('notifies onLocaleChange after resolution and locale changes', async () => {
    const onLocaleChange = vi.fn();
    render(
      <AegisI18nProvider onLocaleChange={onLocaleChange}>
        <SetZhCN />
      </AegisI18nProvider>,
    );

    expect(onLocaleChange).toHaveBeenNthCalledWith(1, 'en');
    await userEvent.click(screen.getByText('set-zh-CN'));
    expect(onLocaleChange).toHaveBeenLastCalledWith('zh-CN');
  });

  it('keeps setLocale stable and changes t only when locale changes', async () => {
    const seenSetLocale = new Set<unknown>();
    const seenTranslate = new Set<unknown>();

    function Capture() {
      const { setLocale, t } = useI18n();
      seenSetLocale.add(setLocale);
      seenTranslate.add(t);
      return <button onClick={() => setLocale('zh-CN')}>set-zh-CN</button>;
    }

    const { rerender } = render(
      <AegisI18nProvider>
        <Capture />
      </AegisI18nProvider>,
    );
    rerender(
      <AegisI18nProvider>
        <Capture />
      </AegisI18nProvider>,
    );

    expect(seenSetLocale.size).toBe(1);
    expect(seenTranslate.size).toBe(1);

    await userEvent.click(screen.getByText('set-zh-CN'));

    expect(seenSetLocale.size).toBe(1);
    expect(seenTranslate.size).toBe(2);
  });

  it('falls back to defaultLocale when storage reads throw', () => {
    const storage = createMemoryStorage();
    vi.spyOn(storage, 'getItem').mockImplementation(() => {
      throw new Error('storage read denied');
    });
    vi.stubGlobal('localStorage', storage);

    render(
      <AegisI18nProvider defaultLocale="zh-CN">
        <ReadI18n />
      </AegisI18nProvider>,
    );

    expect(screen.getByTestId('locale')).toHaveTextContent('zh-CN');
  });

  it('continues rendering, switching, and notifying when storage writes throw', async () => {
    const storage = createMemoryStorage();
    vi.spyOn(storage, 'setItem').mockImplementation(() => {
      throw new Error('storage write denied');
    });
    vi.stubGlobal('localStorage', storage);
    const onLocaleChange = vi.fn();

    render(
      <AegisI18nProvider onLocaleChange={onLocaleChange}>
        <ReadI18n />
        <SetZhCN />
      </AegisI18nProvider>,
    );

    expect(screen.getByTestId('locale')).toHaveTextContent('en');
    expect(onLocaleChange).toHaveBeenCalledWith('en');

    await userEvent.click(screen.getByText('set-zh-CN'));

    expect(screen.getByTestId('locale')).toHaveTextContent('zh-CN');
    expect(onLocaleChange).toHaveBeenLastCalledWith('zh-CN');
  });

  it('throws a clear error when useI18n is called outside the provider', () => {
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => render(<ReadI18n />)).toThrow(
      /useI18n must be used inside <AegisI18nProvider>/,
    );

    errorSpy.mockRestore();
  });
});