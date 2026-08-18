import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useTheme } from '@mui/material/styles';
import { AegisThemeProvider } from './AegisThemeProvider';
import { useThemeMode } from './useThemeMode';

const STORAGE_KEY = 'aegis:theme:mode';

function createMemoryStorage(): Storage {
  // jsdom 25 does not expose a usable `localStorage` global and throws on
  // `new Storage()`. Provide a minimal Storage-shaped shim so provider tests
  // can read/write to it.
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

function ReadThemeMode() {
  const theme = useTheme();
  return <span data-testid="theme-mode">{theme.palette.mode}</span>;
}

function ReadAndSetMode() {
  const { mode, setMode } = useThemeMode();
  return (
    <>
      <span data-testid="hook-mode">{mode}</span>
      <button onClick={() => setMode('dark')}>set-dark</button>
    </>
  );
}

function ReadHookMode() {
  const { mode } = useThemeMode();
  return <span data-testid="hook-mode">{mode}</span>;
}

beforeEach(() => {
  // jsdom 25 leaves `localStorage` as an empty `{}` by default; install a
  // fresh in-memory shim so provider tests can read/write to it.
  vi.stubGlobal('localStorage', createMemoryStorage());
  vi.restoreAllMocks();
});

describe('AegisThemeProvider', () => {
  it('renders children', () => {
    render(
      <AegisThemeProvider>
        <span data-testid="child">child</span>
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('child')).toBeInTheDocument();
  });

  it('default mode is light when localStorage is empty', () => {
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('light');
  });

  it('reads initial mode from localStorage on mount', () => {
    localStorage.setItem(STORAGE_KEY, 'dark');
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('dark');
  });

  it('falls back to light on invalid stored value', () => {
    localStorage.setItem(STORAGE_KEY, 'purple');
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('light');
  });

  it('reads a non-binary theme ID like "totoro" from localStorage and writes it back', () => {
    localStorage.setItem(STORAGE_KEY, 'totoro');
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    // The MUI theme palette.mode is not authoritative for character
    // themes — totoro's palette omits `mode`, so MUI defaults it to
    // 'light'. We assert the localStorage round-trip instead, which
    // is what the Settings page dropdown reads from.
    expect(localStorage.getItem(STORAGE_KEY)).toBe('totoro');
  });

  it('writes the current mode to localStorage on mount', () => {
    render(
      <AegisThemeProvider>
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(localStorage.getItem(STORAGE_KEY)).toBe('light');
  });

  it('mirrors mode changes into the MUI theme', async () => {
    render(
      <AegisThemeProvider>
        <ReadAndSetMode />
        <ReadThemeMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('light');
    await userEvent.click(screen.getByText('set-dark'));
    expect(screen.getByTestId('theme-mode')).toHaveTextContent('dark');
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('dark');
  });

  it('writes the new mode to localStorage when setMode is called', async () => {
    render(
      <AegisThemeProvider>
        <ReadAndSetMode />
      </AegisThemeProvider>,
    );
    expect(localStorage.getItem(STORAGE_KEY)).toBe('light');
    await userEvent.click(screen.getByText('set-dark'));
    expect(localStorage.getItem(STORAGE_KEY)).toBe('dark');
  });

  it('fires onModeChange with the new mode when mode changes', async () => {
    const onModeChange = vi.fn();
    render(
      <AegisThemeProvider onModeChange={onModeChange}>
        <ReadAndSetMode />
      </AegisThemeProvider>,
    );
    await userEvent.click(screen.getByText('set-dark'));
    expect(onModeChange).toHaveBeenCalledWith('dark');
  });

  it('useThemeMode throws when called outside a provider', () => {
    // Suppress the React error boundary noise from the expected throw.
    const errSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => render(<ReadAndSetMode />)).toThrow(
      /useThemeMode must be used inside <AegisThemeProvider>/,
    );
    errSpy.mockRestore();
  });

  it('useThemeMode returns the current mode', () => {
    render(
      <AegisThemeProvider>
        <ReadHookMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('light');
  });

  it('useThemeMode.setMode is stable across renders', () => {
    const seen: Set<unknown> = new Set();
    function Capture() {
      const { setMode } = useThemeMode();
      seen.add(setMode);
      return null;
    }
    const { rerender } = render(
      <AegisThemeProvider>
        <Capture />
      </AegisThemeProvider>,
    );
    rerender(
      <AegisThemeProvider>
        <Capture />
      </AegisThemeProvider>,
    );
    expect(seen.size).toBe(1);
  });

  it('useThemeMode.setMode("dark") updates mode and writes to localStorage', async () => {
    render(
      <AegisThemeProvider>
        <ReadAndSetMode />
      </AegisThemeProvider>,
    );
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('light');
    await userEvent.click(screen.getByText('set-dark'));
    expect(screen.getByTestId('hook-mode')).toHaveTextContent('dark');
    expect(localStorage.getItem(STORAGE_KEY)).toBe('dark');
  });
});
