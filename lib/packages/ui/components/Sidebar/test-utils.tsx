import type { ReactElement } from 'react';
import { render, type RenderOptions } from '@testing-library/react';
import { ThemeProvider, createTheme } from '@mui/material/styles';

const theme = createTheme();

export function renderWithTheme(ui: ReactElement, options?: RenderOptions) {
  return render(<ThemeProvider theme={theme}>{ui}</ThemeProvider>, options);
}

export * from '@testing-library/react';
export { default as userEvent } from '@testing-library/user-event';