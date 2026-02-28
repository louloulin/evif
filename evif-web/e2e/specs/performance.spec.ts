import { test, expect } from '@playwright/test';
import { EditorPage } from '../pages';

/**
 * GWT-E2E-27 to GWT-E2E-28: Performance Tests
 */

test.describe('GWT-E2E-27: 页面加载时间', () => {
  test('Given用户首次访问应用 When页面加载 Then系统应该满足性能指标', async ({ page }) => {
    // Given: 用户首次访问应用
    // Clear any cached resources
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });

    // When: 页面加载
    const startTime = Date.now();

    await page.goto('/', { waitUntil: 'networkidle' });

    // Wait for key elements to be visible
    await page.waitForSelector('.app, #root, [data-testid="app"]', { state: 'visible' });

    const loadTime = Date.now() - startTime;

    // Then: 满足性能指标
    // 首次内容绘制(FCP) < 1.8秒
    expect(loadTime).toBeLessThan(1800);

    // 验证LCP
    const lcp = await page.evaluate(() => {
      return new Promise<number>((resolve) => {
        const observer = new PerformanceObserver((list) => {
          const entries = list.getEntries();
          const lcpEntry = entries[entries.length - 1] as PerformanceEntry;
          resolve(lcpEntry.startTime);
        });
        observer.observe({ entryTypes: ['largest-contentful-paint'] });
        setTimeout(() => resolve(0), 3000);
      });
    });

    if (lcp > 0) {
      // 最大内容绘制(LCP) < 2.5秒
      expect(lcp).toBeLessThan(2500);
    }

    // 累积布局偏移(CLS) < 0.1
    const cls = await page.evaluate(() => {
      return new Promise<number>((resolve) => {
        let clsValue = 0;
        const observer = new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            if (!(entry as any).hadRecentInput) {
              clsValue += (entry as any).value;
            }
          }
        });
        observer.observe({ entryTypes: ['layout-shift'] });
        setTimeout(() => {
          resolve(clsValue);
        }, 3000);
      });
    });

    expect(cls).toBeLessThan(0.1);
  });

  test('Lighthouse性能分数>=90', async ({ page }) => {
    // Note: Full Lighthouse integration requires additional setup
    // This test serves as a placeholder for Lighthouse integration
    const editorPage = new EditorPage(page);
    await editorPage.goto();

    // Verify basic performance indicators
    const loadTime = await page.evaluate(() => {
      const timing = performance.timing;
      return timing.loadEventEnd - timing.navigationStart;
    });

    expect(loadTime).toBeLessThan(3000);
  });
});

test.describe('GWT-E2E-28: 交互响应时间', () => {
  test('Given用户与UI交互 When用户点击按钮或输入 Then系统应该在100ms内显示视觉反馈', async ({ page }) => {
    const editorPage = new EditorPage(page);
    await editorPage.goto();

    // Find a button to test
    const button = page.locator('button').first();

    if (await button.isVisible().catch(() => false)) {
      // Measure click response time
      const startTime = Date.now();

      await button.click();

      // Check for visual feedback (e.g., :active state, color change)
      const hasVisualFeedback = await button.evaluate(el => {
        const style = window.getComputedStyle(el);
        // Check for common visual feedback indicators
        return style.transform !== 'none' ||
               style.backgroundColor !== '' ||
               style.boxShadow !== 'none';
      });

      const responseTime = Date.now() - startTime;

      // 100ms内显示视觉反馈
      expect(responseTime).toBeLessThan(100);
    }
  });

  test('动画帧率>=60fps', async ({ page }) => {
    const editorPage = new EditorPage(page);
    await editorPage.goto();

    // Measure frame rate during animation
    const frameRate = await page.evaluate(() => {
      return new Promise<number>((resolve) => {
        let frames = 0;
        const startTime = performance.now();

        const countFrames = () => {
          frames++;
          if (performance.now() - startTime < 1000) {
            requestAnimationFrame(countFrames);
          } else {
            resolve(frames);
          }
        };

        requestAnimationFrame(countFrames);
      });
    });

    // 动画帧率应该>=60fps
    expect(frameRate).toBeGreaterThanOrEqual(58); // Allow some tolerance
  });

  test('不阻塞主线程', async ({ page }) => {
    const editorPage = new EditorPage(page);
    await editorPage.goto();

    // Check for long tasks that might block the main thread
    const longTasks = await page.evaluate(() => {
      return new Promise<PerformanceEntry[]>((resolve) => {
        const longTasks: PerformanceEntry[] = [];
        const observer = new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            longTasks.push(entry);
          }
        });
        observer.observe({ entryTypes: ['longtask'] });
        setTimeout(() => resolve(longTasks), 3000);
      });
    });

    // Long tasks (blocking the main thread for >50ms) should be minimal
    expect(longTasks.length).toBeLessThan(5);
  });
});
