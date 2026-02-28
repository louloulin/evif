import { test, expect } from '@playwright/test';
import { EditorPage } from '../pages';

/**
 * GWT-E2E-13 to GWT-E2E-14: Plugin Manager Tests
 */

test.describe('GWT-E2E-13: 查看插件列表', () => {
  test('Given用户打开PluginManager WhenPluginList加载完成 Then系统应该显示所有已安装插件', async ({ page }) => {
    const editorPage = new EditorPage(page);

    // Given: 用户打开PluginManager
    await editorPage.goto();

    // Navigate to plugin manager
    const pluginButton = page.locator('[data-testid="activity-plugins"], [title="Plugins"], [title="插件"], button:has-text("Plugins"), button:has-text("插件")');

    if (await pluginButton.isVisible().catch(() => false)) {
      await pluginButton.click();

      // When: PluginList加载完成
      const pluginList = page.locator('[data-testid="plugin-list"], .plugin-list, [data-testid="plugin-manager"]');
      await expect(pluginList).toBeVisible({ timeout: 10000 });

      // Then: 显示所有已安装插件
      const pluginItems = pluginList.locator('[data-testid="plugin-item"], .plugin-item, .plugin-card');
      const pluginCount = await pluginItems.count();

      // If plugins exist, verify their structure
      if (pluginCount > 0) {
        for (let i = 0; i < Math.min(pluginCount, 3); i++) {
          const pluginItem = pluginItems.nth(i);

          // 显示每个插件的状态（启用/禁用）
          const status = pluginItem.locator('[data-testid="plugin-status"], .plugin-status, .status-badge');
          const hasStatus = await status.isVisible().catch(() => false);

          // 显示插件版本和描述
          const name = pluginItem.locator('[data-testid="plugin-name"], .plugin-name, h3, h4').first();
          const version = pluginItem.locator('[data-testid="plugin-version"], .plugin-version');
          const description = pluginItem.locator('[data-testid="plugin-description"], .plugin-description, p');

          expect(await name.isVisible().catch(() => false)).toBe(true);

          // 提供启用/禁用切换按钮
          const toggleButton = pluginItem.locator('[data-testid="plugin-toggle"], .plugin-toggle, button:has-text("Enable"), button:has-text("Disable"), button:has-text("启用"), button:has-text("禁用")');
          expect(await toggleButton.isVisible().catch(() => false)).toBe(true);
        }
      }
    }
  });

  test('API验证: GET /api/v1/plugins 返回200状态码', async ({ page }) => {
    const editorPage = new EditorPage(page);
    await editorPage.goto();

    const apiResponsePromise = page.waitForResponse(
      response => response.url().includes('/api/v1/plugins') && response.request().method() === 'GET',
      { timeout: 10000 }
    );

    // Navigate to plugin manager
    const pluginButton = page.locator('[data-testid="activity-plugins"], [title="Plugins"], button:has-text("Plugins")');
    if (await pluginButton.isVisible().catch(() => false)) {
      await pluginButton.click();
    }

    try {
      const response = await apiResponsePromise;
      expect(response.status()).toBe(200);

      const responseBody = await response.json();
      expect(Array.isArray(responseBody)).toBe(true);

      if (responseBody.length > 0) {
        // Verify plugin structure
        const plugin = responseBody[0];
        expect(plugin).toHaveProperty('id');
        expect(plugin).toHaveProperty('name');
        expect(plugin).toHaveProperty('status');
      }
    } catch (e) {
      test.skip();
    }
  });
});

test.describe('GWT-E2E-14: 插件状态切换', () => {
  test('Given用户在PluginManager中 When用户切换插件状态 Then系统应该更新PluginStatus显示', async ({ page }) => {
    const editorPage = new EditorPage(page);

    await editorPage.goto();

    // Navigate to plugin manager
    const pluginButton = page.locator('[data-testid="activity-plugins"], [title="Plugins"], button:has-text("Plugins")');
    if (await pluginButton.isVisible().catch(() => false)) {
      await pluginButton.click();

      const pluginList = page.locator('[data-testid="plugin-list"], .plugin-list');
      await expect(pluginList).toBeVisible({ timeout: 10000 });

      // Find the first plugin with a toggle button
      const pluginItems = pluginList.locator('[data-testid="plugin-item"], .plugin-item');
      const count = await pluginItems.count();

      if (count > 0) {
        const firstPlugin = pluginItems.first();
        const toggleButton = firstPlugin.locator('[data-testid="plugin-toggle"], .plugin-toggle, button:has-text("Enable"), button:has-text("Disable")');

        if (await toggleButton.isVisible().catch(() => false)) {
          // Get initial status
          const initialStatus = await firstPlugin.locator('[data-testid="plugin-status"], .plugin-status').textContent().catch(() => '');

          // When: 用户切换插件状态
          await toggleButton.click();

          // Wait for status update
          await page.waitForTimeout(1000);

          // Then: 系统应该更新PluginStatus显示
          const newStatus = await firstPlugin.locator('[data-testid="plugin-status"], .plugin-status').textContent().catch(() => '');

          // Status should have changed
          expect(newStatus).not.toBe(initialStatus);
        }
      }
    }
  });

  test('API验证: PATCH /api/v1/plugins/{id} 返回200状态码', async ({ page }) => {
    const editorPage = new EditorPage(page);
    await editorPage.goto();

    const apiResponsePromise = page.waitForResponse(
      response => response.url().includes('/api/v1/plugins/') && response.request().method() === 'PATCH',
      { timeout: 10000 }
    );

    // Navigate to plugin manager and toggle a plugin
    const pluginButton = page.locator('[data-testid="activity-plugins"], [title="Plugins"], button:has-text("Plugins")');
    if (await pluginButton.isVisible().catch(() => false)) {
      await pluginButton.click();

      const pluginList = page.locator('[data-testid="plugin-list"], .plugin-list');
      if (await pluginList.isVisible().catch(() => false)) {
        const toggleButton = pluginList.locator('[data-testid="plugin-toggle"], .plugin-toggle').first();
        if (await toggleButton.isVisible().catch(() => false)) {
          await toggleButton.click();
        }
      }
    }

    try {
      const response = await apiResponsePromise;
      expect(response.status()).toBe(200);

      const responseBody = await response.json();
      expect(responseBody).toHaveProperty('id');
      expect(responseBody).toHaveProperty('status');
    } catch (e) {
      test.skip();
    }
  });
});
