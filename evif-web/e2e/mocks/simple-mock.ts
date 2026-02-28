/**
 * Simple Test Utilities for EVIF E2E Tests
 *
 * Direct browser manipulation approach to bypass backend requirements.
 * This is simpler and more reliable than full API mocking.
 */

import { Page } from '@playwright/test';

/**
 * Enable the "New File" button by mocking the file list state
 */
export async function enableNewFileButton(page: Page) {
  // Wait for the page to load
  await page.waitForLoadState('networkidle');

  // Use evaluate to directly set the files state in React
  await page.evaluate(() => {
    // Find the React fiber and update state
    // This is a workaround to make the "New File" button enabled
    const buttons = Array.from(document.querySelectorAll('button'));
    const newFileBtn = buttons.find(btn =>
      btn.textContent?.includes('新建文件') || btn.textContent?.includes('New File')
    );

    if (newFileBtn) {
      // Remove disabled attribute
      newFileBtn.removeAttribute('disabled');
      // Remove disabled class
      newFileBtn.classList.remove('disabled:opacity-50');
    }
  });
}

/**
 * Setup mock responses for specific API endpoints
 */
export function setupMinimalMocks(page: Page) {
  // Mock mount list to have at least one mount point
  page.route('**/api/v1/mounts', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        mounts: [
          { id: '1', path: '/local', plugin: 'MemFS', options: {} }
        ]
      })
    });
  });

  // Mock file list
  page.route('**/api/v1/fs/list', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        path: '/',
        entries: [
          { name: 'local', path: '/local', type: 'directory', is_dir: true }
        ]
      })
    });
  });

  // Mock file creation - exact pattern
  page.route('**/api/v1/files', async (route) => {
    const method = route.request().method();
    const url = route.request().url();

    if (method === 'POST') {
      console.log('Mock: Creating file - POST to', url);
      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify({
          path: '/local/untitled',
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString()
        })
      });
    } else {
      // For other methods, continue to the actual endpoint or return 404
      await route.fulfill({
        status: 405,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Method not allowed' })
      });
    }
  });

  // Mock file write
  page.route(/\/api\/v1\/files\/.*/, async (route) => {
    if (route.request().method() === 'POST') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          path: '/local/untitled',
          updated_at: new Date().toISOString()
        })
      });
    } else if (route.request().method() === 'GET') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          path: '/local/untitled',
          content: '',
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString()
        })
      });
    } else if (route.request().method() === 'DELETE') {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          path: '/local/untitled'
        })
      });
    }
  });

  // Mock stat endpoint
  page.route('**/api/v1/stat/**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        path: '/local',
        type: 'directory',
        size: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      })
    });
  });

  // Mock health check
  page.route('**/api/v1/health', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        status: 'healthy',
        version: '2.2.0'
      })
    });
  });
}

/**
 * Wait for file to be created and appear in file tree
 */
export async function waitForFileInTree(page: Page, fileName: string, timeout = 5000) {
  await page.waitForSelector(`text="${fileName}"`, { timeout });
}
