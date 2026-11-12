import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './test',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 20_000,
  reporter: 'list',
  use: { headless: true, trace: 'off', screenshot: 'off', video: 'off', serviceWorkers: 'block' },
});
