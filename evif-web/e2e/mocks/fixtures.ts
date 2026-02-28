/**
 * Test Fixtures with Mock API
 *
 * Custom Playwright test fixtures that automatically setup mock API routes.
 * Tests can now run independently of backend state.
 */

import { test as base, Page } from '@playwright/test';
import { setupMinimalMocks, enableNewFileButton } from './simple-mock';

// Extend base test with mock API support
export const test = base.extend<{
  page: Page;
}>({
  // Override page fixture to setup mocks automatically
  page: async ({ page }, use) => {
    // Setup minimal mock API routes
    setupMinimalMocks(page);

    // Use the page in tests
    await use(page);
  },
});

// Re-export everything from Playwright
export * from '@playwright/test';
export { expect } from '@playwright/test';
export { enableNewFileButton } from './simple-mock';
