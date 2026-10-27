import { describe, expect, it } from 'vitest';

import { PieChart } from './index';

describe('@aegis/ui/charts', () => {
  it('re-exports the x-charts pie primitives', () => {
    expect(PieChart).toBeDefined();
  });
});
