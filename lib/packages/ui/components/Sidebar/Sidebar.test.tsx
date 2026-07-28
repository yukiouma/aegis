import { describe, it, expect, vi } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Sidebar } from './Sidebar';
import type { MenuItem } from './types';
import { renderWithTheme } from './test-utils';

const Icon = () => <svg data-testid="mock-icon" />;

const baseMenu: MenuItem[] = [
  { link: '/home', title: 'Home', icon: Icon },
  {
    link: '/settings',
    title: 'Settings',
    icon: Icon,
    subMenu: [{ link: '/settings/profile', title: 'Profile', icon: Icon }],
  },
];

const defaultProps = {
  title: 'My App',
  menu: baseMenu,
  open: true,
  onToggle: () => {},
};

describe('Sidebar', () => {
  it('renders title when open=true', () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    expect(screen.getByText('My App')).toBeInTheDocument();
  });

  it('hides title when open=false', () => {
    renderWithTheme(<Sidebar {...defaultProps} open={false} />);
    expect(screen.queryByText('My App')).not.toBeInTheDocument();
  });

  it('toggle button calls onToggle when clicked', async () => {
    const onToggle = vi.fn();
    renderWithTheme(<Sidebar {...defaultProps} onToggle={onToggle} />);
    await userEvent.click(screen.getByLabelText('toggle sidebar'));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it('toggle icon is FormatIndentDecrease when open=true', () => {
    renderWithTheme(<Sidebar {...defaultProps} />);
    expect(screen.getByTestId('FormatIndentDecreaseIcon')).toBeInTheDocument();
  });

  it('toggle icon is FormatIndentIncrease when open=false', () => {
    renderWithTheme(<Sidebar {...defaultProps} open={false} />);
    expect(screen.getByTestId('FormatIndentIncreaseIcon')).toBeInTheDocument();
  });
});