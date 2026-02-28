/**
 * MSW (Mock Service Worker) Setup for Playwright E2E Tests
 *
 * This file sets up API mocking for the EVIF web application to make
 * E2E tests independent of backend state. Tests can now run without
 * requiring a configured backend or mount points.
 *
 * Benefits:
 * - Tests are faster and more reliable
 * - No infrastructure dependencies
 * - Tests focus on UI behavior, not backend integration
 * - Backend API testing should be separate unit/integration tests
 */

import { worker } from './mocks/browser';

// Add custom fixture to initialize MSW
import { test as base } from '@playwright/test';

export const test = base.extend<{
  initializeMocks: () => Promise<void>;
}>({
  // Custom fixture to initialize MSW worker
  initializeMocks: async ({ page }, use) => {
    // Inject MSW worker into the page
    await page.addInitScript(() => {
      // This will be replaced with the actual worker code
      window.mswWorker = null;
    });

    // Wait for MSW to be ready
    await page.goto('/');

    // Inject and start MSW worker
    await page.evaluate(async () => {
      // Dynamic import of MSW worker code
      const { worker } = await import('./mocks/browser');
      await worker.start({
        onUnhandledRequest: 'warn'
      });
    });

    await use();
  };
});

// Re-export everything from Playwright
export * from '@playwright/test';
export { expect } from '@playwright/test';
