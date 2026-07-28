import { describe, it, expect } from 'vitest';
import * as Mui from '@mui/material';
import * as Icons from '@mui/icons-material';
import { mui, icons } from './index';

describe('barrel re-exports', () => {
  it('mui barrel re-exports @mui/material', () => {
    expect(mui.Button).toBe(Mui.Button);
  });

  it('icons barrel re-exports @mui/icons-material', () => {
    expect(icons.Home).toBe(Icons.Home);
  });
});