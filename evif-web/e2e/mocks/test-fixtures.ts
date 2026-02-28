import { test as base } from '@playwright/test';
import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';
import { handlers } from './handlers';

// MSW setup
export const mockServer = setupServer(...handlers);

export const test = base.extend<{
  mockServer: typeof mockServer;
}>({
  // Initialize mock server before each test
  mockServer: async ({}, use) => {
    // Start the mock server
    mockServer.listen({
      onUnhandledRequest: 'warn'
    });

    // Use the mock server in the test
    await use(mockServer);

    // Clean up after each test
    mockServer.resetHandlers();
    mockServer.close();
  }
});

// Re-export everything from Playwright
export * from '@playwright/test';
export { expect } from '@playwright/test';
