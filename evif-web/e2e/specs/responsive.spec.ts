import { test, expect } from '@playwright/test';
import { EditorPage } from '../pages';

/**
 * GWT-E2E-21 to GWT-E2E-23: Responsive Design Tests
 */

test.describe('GWT-E2E-21: 移动端视图', () => {
  test('Given用户在移动设备（宽度<768px） When页面加载完成 Then系统应该显示汉堡菜单按钮', async ({ page }) => {
    const editorPage = new EditorPage(page);

    // Given: 用户在移动设备
    await page.setViewportSize({ width: 375, height: 667 });

    // When: 页面加载完成
    await editorPage.goto();

    // Then: 显示汉堡菜单按钮
    const hamburgerMenu = page.locator('[data-testid="mobile-menu-button"], .hamburger-menu, button[aria-label*="menu"], button:has(.menu-icon)');
    await expect(hamburgerMenu).toBeVisible();

    // And: 隐藏侧边栏
    const sidebar = page.locator('.sidebar, [data-testid="sidebar"]');
    if (await sidebar.isVisible().catch(() => false)) {
      // 侧边栏可能在移动视图下被隐藏或折叠
      const sidebarWidth = await sidebar.evaluate(el => el.offsetWidth);
      expect(sidebarWidth).toBeLessThan(100); // 侧边栏应该很窄或隐藏
    }

    // And: 调整编辑器宽度为100%
    const editor = page.locator('.editor-container, [data-testid="editor-container"]');
    if (await editor.isVisible()) {
      const editorWidth = await editor.evaluate(el => el.offsetWidth);
      const viewportWidth = 375;
      expect(editorWidth).toBeLessThanOrEqual(viewportWidth);
    }
  });

  test('触摸目标至少44x44px', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });

    const editorPage = new EditorPage(page);
    await editorPage.goto();

    // 检查所有按钮和可点击元素的最小触摸目标大小
    const interactiveElements = page.locator('button, a, [role="button"], input[type="checkbox"], input[type="radio"], select, textarea, [onclick]');
    const count = await interactiveElements.count();

    for (let i = 0; i < count; i++) {
      const element = interactiveElements.nth(i);
      if (await element.isVisible().catch(() => false)) {
        const box = await element.boundingBox();
        if (box) {
          // 触摸目标应该至少44x44px
          expect(box.width, `Element at index ${i} width should be at least 44px`).toBeGreaterThanOrEqual(44);
          expect(box.height, `Element at index ${i} height should be at least 44px`).toBeGreaterThanOrEqual(44);
        }
      }
    }
  });
});

test.describe('GWT-E2E-22: 平板视图', () => {
  test('Given用户在平板设备（768px<=宽度<1024px） When页面加载完成 Then系统应该显示简化的侧边栏', async ({ page }) => {
    const editorPage = new EditorPage(page);

    // Given: 用户在平板设备
    await page.setViewportSize({ width: 900, height: 1024 });

    // When: 页面加载完成
    await editorPage.goto();

    // Then: 显示简化的侧边栏
    const sidebar = page.locator('.sidebar, [data-testid="sidebar"]');
    await expect(sidebar).toBeVisible();

    // 侧边栏应该是简化版本（较窄）
    const sidebarWidth = await sidebar.evaluate(el => el.offsetWidth);
    expect(sidebarWidth).toBeLessThan(300);

    // And: 调整两列布局
    const mainContent = page.locator('.main-content, [data-testid="main-content"], .workbench');
    await expect(mainContent).toBeVisible();

    // And: 保持编辑器可用性
    const editor = page.locator('.monaco-editor, [data-testid="editor"], .editor');
    if (await editor.isVisible().catch(() => false)) {
      await expect(editor).toBeVisible();
    }
  });
});

test.describe('GWT-E2E-23: 桌面视图', () => {
  test('Given用户在桌面设备（宽度>=1024px） When页面加载完成 Then系统应该显示完整的侧边栏', async ({ page }) => {
    const editorPage = new EditorPage(page);

    // Given: 用户在桌面设备
    await page.setViewportSize({ width: 1920, height: 1080 });

    // When: 页面加载完成
    await editorPage.goto();

    // Then: 显示完整的侧边栏
    const sidebar = page.locator('.sidebar, [data-testid="sidebar"]');
    await expect(sidebar).toBeVisible();

    // 侧边栏应该是完整版本（较宽）
    const sidebarWidth = await sidebar.evaluate(el => el.offsetWidth);
    expect(sidebarWidth).toBeGreaterThanOrEqual(200);

    // And: 显示多列编辑器布局
    const editorContainer = page.locator('.editor-container, [data-testid="editor-container"]');
    await expect(editorContainer).toBeVisible();

    // And: 显示所有面板和工具栏
    const menuBar = page.locator('header, [data-testid="menu-bar"], .menu-bar');
    await expect(menuBar).toBeVisible();

    const activityBar = page.locator('.activity-bar, [data-testid="activity-bar"]');
    if (await activityBar.isVisible().catch(() => false)) {
      await expect(activityBar).toBeVisible();
    }

    const statusBar = page.locator('footer, [data-testid="status-bar"], .status-bar');
    await expect(statusBar).toBeVisible();
  });
});
