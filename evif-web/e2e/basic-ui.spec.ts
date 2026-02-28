import { test, expect } from '@playwright/test';

test.describe('EVIF Web UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should load the main application', async ({ page }) => {
    // Check that main elements are present
    await expect(page.locator('.app')).toBeVisible();
    await expect(page.locator('.workbench')).toBeVisible();
  });

  test('should display menu bar', async ({ page }) => {
    await expect(page.locator('header')).toBeVisible();
  });

  test('should display activity bar', async ({ page }) => {
    await expect(page.locator('.activity-bar')).toBeVisible();
  });

  test('should display sidebar', async ({ page }) => {
    await expect(page.locator('.sidebar')).toBeVisible();
  });

  test('should display status bar', async ({ page }) => {
    await expect(page.locator('footer')).toBeVisible();
  });

  test('should show empty state when no files are open', async ({ page }) => {
    const editorContainer = page.locator('.editor-container');
    // Check for Chinese text (localized UI)
    await expect(editorContainer).toContainText('未打开文件');
  });
});

test.describe('File Tree', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should display file tree component', async ({ page }) => {
    const fileTree = page.locator('.sidebar').locator('text=/explorer|files/i');
    await expect(fileTree).toBeVisible();
  });
});

test.describe('Editor Tabs', () => {
  test('should not show tabs when no files are open', async ({ page }) => {
    await page.goto('/');
    // Check for Chinese text (localized UI) - "未打开文件" means "No file open"
    const tabsContainer = page.locator('.editor-container').locator('text=/未打开文件/');
    await expect(tabsContainer).toBeVisible();
  });
});

test.describe('Panel', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
  });

  test('should display bottom panel', async ({ page }) => {
    const panel = page.locator('.panel');
    await expect(panel).toBeVisible();
  });

  test('should show terminal tab by default', async ({ page }) => {
    const terminalTab = page.locator('text=/terminal/i');
    await expect(terminalTab).toBeVisible();
  });
});

test.describe('Activity Bar Navigation', () => {
  test('should switch between views', async ({ page }) => {
    await page.goto('/');

    // Click on plugins icon if available
    const pluginsButton = page.locator('[title="Plugins"], [aria-label="Plugins"]');
    if (await pluginsButton.isVisible()) {
      await pluginsButton.click();
      await expect(page.locator('text=/plugin/i')).toBeVisible();
    }
  });
});

test.describe('Responsive Design', () => {
  test('should adapt to smaller screens', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 1024 });
    await page.goto('/');

    const app = page.locator('.app');
    await expect(app).toBeVisible();
  });

  test('should adapt to mobile screens', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');

    const app = page.locator('.app');
    await expect(app).toBeVisible();
  });
});
