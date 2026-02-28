# E2E Testing with Playwright

This directory contains end-to-end tests for the EVIF Web UI using Playwright.

## Setup

Install dependencies:
```bash
npm install
```

Install Playwright browsers:
```bash
npx playwright install
```

## Running Tests

Run all tests:
```bash
npm run test:e2e
```

Run tests in UI mode (recommended for development):
```bash
npm run test:e2e:ui
```

Run tests in headed mode (see browser window):
```bash
npm run test:e2e:headed
```

Debug tests:
```bash
npm run test:e2e:debug
```

## Test Structure

- `basic-ui.spec.ts` - Tests for basic UI components and layout
- `editor-tabs.spec.ts` - Tests for multi-tab editing functionality
- `file-operations.spec.ts` - Tests for file operations (open, save, delete)

## Writing New Tests

1. Create a new spec file in this directory
2. Use `test.describe()` to group related tests
3. Use `test()` to define individual test cases
4. Use Playwright's locators to find elements:
   - `page.locator('.class')` - CSS selector
   - `page.locator('text=Text')` - Text selector
   - `page.getByRole('button')` - ARIA role

## Best Practices

- Keep tests focused and independent
- Use data-testid attributes for reliable element selection
- Wait for elements to be visible before interacting
- Use page objects for complex interactions
- Run tests in CI with headed=false

## Troubleshooting

If tests fail:
1. Check if the dev server is running on port 3000
2. Verify backend API is accessible
3. Run with `--headed` flag to see what's happening
4. Use `--debug` to step through tests
5. Check screenshots in `test-results/`
