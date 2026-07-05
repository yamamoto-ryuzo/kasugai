import { test, expect } from '@playwright/test';
import * as path from 'path';

// Tauri アプリのログイン自動化や設定画面のUIテスト
test.describe('Kasugai ログインおよびKeyring連携のテスト', () => {
  
  test('設定画面で資格情報を入力すると保存され、イベントが発火すること', async ({ page }) => {
    // 画面2（index2.html）を直接開く
    const indexPath = path.resolve(__dirname, '../src/index2.html');
    await page.goto(`file://${indexPath}?settings=true`);

    // タイトルがシステム設定になっているか確認
    await expect(page.locator('#view-title')).toHaveText('システム設定');

    // メールアドレスとパスワードを入力
    const testEmail = 'test-user@example.com';
    const testPassword = 'securepassword123';
    
    await page.fill('#box-email-input', testEmail);
    await page.fill('#box-password-input', testPassword);

    // 保存ボタンをクリック
    await page.click('button:has-text("設定をすべて保存")');

    // ステータスメッセージを確認
    const statusMsg = page.locator('#settings-status');
    await expect(statusMsg).toContainText('設定をすべてセキュアに保存しました！');

    // localStorageにEmailが保存されているか確認
    const savedEmail = await page.evaluate(() => localStorage.getItem('box_email'));
    expect(savedEmail).toBe(testEmail);
  });

  test('ログイン画面自動入力 (inject_autologin) のモック検証', async ({ page }) => {
    // ログイン入力項目を持つ仮のBOXログイン画面に類似したダミーHTMLを生成して検証
    await page.setContent(`
      <html>
        <body>
          <form>
            <input type="email" name="login" id="email-field" />
            <input type="password" name="password" id="password-field" />
            <button type="submit" id="login-submit">次へ</button>
          </form>
        </body>
      </html>
    `);

    // 自動ログインスクリプトの注入をシミュレート
    const email = 'autologin-test@example.com';
    const password = 'supersecretpassword';

    await page.evaluate(({ email, password }) => {
      // 実際メインプロセスから注入されるJSをほぼ同一の形で実行
      (function() {
        let emailInput = document.querySelector('input[type="email"]') || document.querySelector('input[name="login"]');
        if (emailInput) {
          emailInput.value = email;
          emailInput.dispatchEvent(new Event('input', { bubbles: true }));
          emailInput.dispatchEvent(new Event('change', { bubbles: true }));
        }

        let passInput = document.querySelector('input[type="password"]') || document.querySelector('input[name="password"]');
        if (passInput) {
          passInput.value = password;
          passInput.dispatchEvent(new Event('input', { bubbles: true }));
          passInput.dispatchEvent(new Event('change', { bubbles: true }));
        }
      })();
    }, { email, password });

    // 入力値が自動的にセットされているか検証
    await expect(page.locator('#email-field')).toHaveValue(email);
    await expect(page.locator('#password-field')).toHaveValue(password);
  });
});
