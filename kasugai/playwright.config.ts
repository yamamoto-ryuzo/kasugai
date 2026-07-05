import { defineConfig, devices } from '@playwright/test';

// Tauriアプリの自動テスト用のPlaywright設定
export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1, // Tauriアプリテストは1ワーカー推奨
  reporter: 'html',
  use: {
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'tauri-app',
      use: {
        ...devices['Desktop Chrome'],
        // Tauriアプリ固有の設定や環境変数をここに指定できます
      },
    },
  ],
});
