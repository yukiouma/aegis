import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/helpers/setup.ts'],
    globals: false,
    passWithNoTests: true,
  },
});