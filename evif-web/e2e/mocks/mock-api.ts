/**
 * Mock API Setup for Playwright E2E Tests
 *
 * Uses Playwright's built-in request interception to mock backend API responses.
 * This makes tests independent of backend state and faster to execute.
 */

import { Page } from '@playwright/test';

// Mock data store
const mockFiles = new Map<string, { content: string; created_at: string; updated_at: string }>();
const mockMounts = [
  { id: '1', path: '/local', plugin: 'MemFS', options: {} },
  { id: '2', path: '/temp', plugin: 'MemFS', options: {} }
];

let nextFileId = 1;

// Helper to get current timestamp
const now = () => new Date().toISOString();

// Reset mock state between tests
export function resetMockState() {
  mockFiles.clear();
  // Reset mounts to defaults
  mockMounts.length = 0;
  mockMounts.push(
    { id: '1', path: '/local', plugin: 'MemFS', options: {} },
    { id: '2', path: '/temp', plugin: 'MemFS', options: {} }
  );
  nextFileId = 1;
}

// Setup mock API routes for a page
export function setupMockAPI(page: Page) {
  // GET /api/v1/mounts - List all mount points
  page.route('**/api/v1/mounts', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ mounts: mockMounts })
    });
  });

  // POST /api/v1/mount - Create a new mount point
  page.route('**/api/v1/mount', async (route) => {
    const body = await route.request().postDataJSON();
    const newMount = {
      id: String(nextFileId++),
      path: body.path,
      plugin: body.plugin,
      options: body.options || {}
    };
    mockMounts.push(newMount);
    await route.fulfill({
      status: 201,
      contentType: 'application/json',
      body: JSON.stringify(newMount)
    });
  });

  // POST /api/v1/unmount - Remove a mount point
  page.route('**/api/v1/unmount', async (route) => {
    const body = await route.request().postDataJSON();
    const index = mockMounts.findIndex(m => m.path === body.path);
    if (index !== -1) {
      mockMounts.splice(index, 1);
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ success: true })
    });
  });

  // POST /api/v1/files - Create a new file
  page.route('**/api/v1/files', async (route) => {
    if (route.request().method() === 'POST') {
      const body = await route.request().postDataJSON() as { path?: string };
      const filePath = body.path || `/local/untitled`;

      mockFiles.set(filePath, {
        content: '',
        created_at: now(),
        updated_at: now()
      });

      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify({
          path: filePath,
          created_at: mockFiles.get(filePath)!.created_at,
          updated_at: mockFiles.get(filePath)!.updated_at
        })
      });
    } else {
      await route.continue();
    }
  });

  // POST /api/v1/files/:path - Write file content
  page.route(/\/api\/v1\/files\/.*/, async (route) => {
    const url = new URL(route.request().url());
    const filePath = '/' + url.pathname.split('/').slice(4).join('/');

    if (route.request().method() === 'POST') {
      const body = await route.request().postDataJSON() as { content?: string; base64?: boolean };
      let content = body.content || '';

      if (!mockFiles.has(filePath)) {
        mockFiles.set(filePath, {
          content,
          created_at: now(),
          updated_at: now()
        });
      } else {
        const file = mockFiles.get(filePath)!;
        file.content = content;
        file.updated_at = now();
      }

      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          path: filePath,
          updated_at: mockFiles.get(filePath)!.updated_at
        })
      });
    } else if (route.request().method() === 'GET') {
      // GET /api/v1/files/:path - Read file content
      const base64 = url.searchParams.get('base64') === 'true';

      if (!mockFiles.has(filePath)) {
        await route.fulfill({
          status: 404,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'File not found', path: filePath })
        });
        return;
      }

      const file = mockFiles.get(filePath)!;
      let content = file.content;

      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          path: filePath,
          content,
          created_at: file.created_at,
          updated_at: file.updated_at
        })
      });
    } else if (route.request().method() === 'DELETE') {
      // DELETE /api/v1/files/:path
      if (!mockFiles.has(filePath)) {
        await route.fulfill({
          status: 404,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'File not found', path: filePath })
        });
        return;
      }

      mockFiles.delete(filePath);
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          path: filePath
        })
      });
    } else {
      await route.continue();
    }
  });

  // GET /api/v1/directories/list/:path - List directory contents
  page.route(/\/api\/v1\/directories\/list\/.*/, async (route) => {
    const url = new URL(route.request().url());
    const dirPath = '/' + url.pathname.split('/').slice(5).join('/');

    const entries: Array<{ name: string; path: string; type: 'file' | 'directory' }> = [];

    // Add default entries for root and mount points
    if (dirPath === '/' || dirPath === '/local' || dirPath === '/temp') {
      for (const mount of mockMounts) {
        entries.push({
          name: mount.path.substring(1),
          path: mount.path,
          type: 'directory'
        });
      }
    }

    // Add files that exist in this directory
    for (const [filePath, file] of mockFiles.entries()) {
      if (filePath.startsWith(dirPath + '/') || filePath.startsWith(dirPath)) {
        const relativePath = filePath.substring(dirPath.length + 1);
        const parts = relativePath.split('/');

        if (parts.length === 1 && parts[0]) {
          entries.push({
            name: parts[0],
            path: filePath,
            type: 'file'
          });
        }
      }
    }

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        path: dirPath,
        entries
      })
    });
  });

  // GET /api/v1/fs/list - List directory contents (new endpoint used by frontend)
  page.route('**/api/v1/fs/list', async (route) => {
    const url = new URL(route.request().url());
    const dirPath = url.searchParams.get('path') || '/';

    const entries: Array<{ name: string; path: string; type: 'file' | 'directory'; is_dir: boolean }> = [];

    // Add mount points as directories
    for (const mount of mockMounts) {
      entries.push({
        name: mount.path.substring(1), // Remove leading slash
        path: mount.path,
        type: 'directory',
        is_dir: true
      });
    }

    // Add files that exist in this directory
    for (const [filePath, file] of mockFiles.entries()) {
      if (filePath.startsWith(dirPath + '/') || filePath.startsWith(dirPath)) {
        const relativePath = filePath.substring(dirPath.length + 1);
        const parts = relativePath.split('/');

        if (parts.length === 1 && parts[0]) {
          entries.push({
            name: parts[0],
            path: filePath,
            type: 'file',
            is_dir: false
          });
        }
      }
    }

    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        path: dirPath,
        entries
      })
    });
  });

  // GET /api/v1/stat/:path - Get file/directory stats
  page.route(/\/api\/v1\/stat\/.*/, async (route) => {
    const url = new URL(route.request().url());
    const filePath = '/' + url.pathname.split('/').slice(4).join('/');

    if (mockFiles.has(filePath)) {
      const file = mockFiles.get(filePath)!;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          path: filePath,
          type: 'file',
          size: file.content.length,
          created_at: file.created_at,
          updated_at: file.updated_at
        })
      });
      return;
    }

    // Check if it's a mount point
    const mount = mockMounts.find(m => m.path === filePath);
    if (mount) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          path: filePath,
          type: 'directory',
          size: 0,
          created_at: now(),
          updated_at: now()
        })
      });
      return;
    }

    await route.fulfill({
      status: 404,
      contentType: 'application/json',
      body: JSON.stringify({ error: 'Not found', path: filePath })
    });
  });

  // GET /api/v1/plugins - List all plugins
  page.route('**/api/v1/plugins', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        plugins: [
          { name: 'LocalFS', version: '0.1.0', enabled: true },
          { name: 'MemFS', version: '0.1.0', enabled: true },
          { name: 'HTTPFS', version: '0.1.0', enabled: true },
          { name: 'S3FS', version: '0.1.0', enabled: false },
          { name: 'KVFS', version: '0.1.0', enabled: true }
        ]
      })
    });
  });

  // Health check endpoints
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

  page.route('**/health', async (route) => {
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
