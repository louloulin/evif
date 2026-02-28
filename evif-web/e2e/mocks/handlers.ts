import { http, HttpResponse } from 'msw';

// Mock data store
const mockFiles = new Map<string, { content: string; created_at: string; updated_at: string }>();
const mockMounts = [
  { id: '1', path: '/local', plugin: 'MemFS', options: {} },
  { id: '2', path: '/temp', plugin: 'MemFS', options: {} }
];

let nextFileId = 1;

// Helper to get current timestamp
const now = () => new Date().toISOString();

// Mount point handlers
export const mountHandlers = [
  // GET /api/v1/mounts - List all mount points
  http.get('http://localhost:3000/api/v1/mounts', () => {
    return HttpResponse.json({
      mounts: mockMounts
    });
  }),

  // POST /api/v1/mount - Create a new mount point
  http.post('http://localhost:3000/api/v1/mount', async ({ request }) => {
    const body = await request.json() as { path: string; plugin: string; options?: Record<string, unknown> };
    const newMount = {
      id: String(nextFileId++),
      path: body.path,
      plugin: body.plugin,
      options: body.options || {}
    };
    mockMounts.push(newMount);
    return HttpResponse.json(newMount, { status: 201 });
  }),

  // POST /api/v1/unmount - Remove a mount point
  http.post('http://localhost:3000/api/v1/unmount', async ({ request }) => {
    const body = await request.json() as { path: string };
    const index = mockMounts.findIndex(m => m.path === body.path);
    if (index !== -1) {
      mockMounts.splice(index, 1);
    }
    return HttpResponse.json({ success: true });
  })
];

// File operation handlers
export const fileHandlers = [
  // POST /api/v1/files - Create a new file
  http.post('http://localhost:3000/api/v1/files', async ({ request }) => {
    const body = await request.json() as { path?: string };
    const filePath = body.path || `/local/untitled`;

    mockFiles.set(filePath, {
      content: '',
      created_at: now(),
      updated_at: now()
    });

    return HttpResponse.json({
      path: filePath,
      created_at: mockFiles.get(filePath)!.created_at,
      updated_at: mockFiles.get(filePath)!.updated_at
    }, { status: 201 });
  }),

  // POST /api/v1/files/:path - Write file content
  http.post<{ path: string }>('http://localhost:3000/api/v1/files/:path', async ({ params, request }) => {
    const filePath = `/${params.path}`;
    const body = await request.json() as { content?: string; base64?: boolean };

    let content = body.content || '';

    // Handle base64 encoding if requested
    if (body.base64 && content) {
      content = Buffer.from(content).toString('base64');
    }

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

    return HttpResponse.json({
      path: filePath,
      updated_at: mockFiles.get(filePath)!.updated_at
    }, { status: 200 });
  }),

  // GET /api/v1/files/:path - Read file content
  http.get<{ path: string }>('http://localhost:3000/api/v1/files/:path', async ({ params, request }) => {
    const filePath = `/${params.path}`;
    const url = new URL(request.url);
    const base64 = url.searchParams.get('base64') === 'true';

    if (!mockFiles.has(filePath)) {
      return HttpResponse.json(
        { error: 'File not found', path: filePath },
        { status: 404 }
      );
    }

    const file = mockFiles.get(filePath)!;
    let content = file.content;

    if (base64 && content) {
      content = Buffer.from(content).toString('base64');
    }

    return HttpResponse.json({
      path: filePath,
      content,
      created_at: file.created_at,
      updated_at: file.updated_at
    });
  }),

  // DELETE /api/v1/files/:path - Delete a file
  http.delete<{ path: string }>('http://localhost:3000/api/v1/files/:path', () => {
    const filePath = `/${params.path}`;

    if (!mockFiles.has(filePath)) {
      return HttpResponse.json(
        { error: 'File not found', path: filePath },
        { status: 404 }
      );
    }

    mockFiles.delete(filePath);
    return HttpResponse.json({
      success: true,
      path: filePath
    });
  }),

  // GET /api/v1/directories/list/:path - List directory contents
  http.get<{ path: string }>('http://localhost:3000/api/v1/directories/list/:path', () => {
    const dirPath = `/${params.path}`;

    // Filter files that start with this directory path
    const entries: Array<{ name: string; path: string; type: 'file' | 'directory' }> = [];

    // Add default entries for /local
    if (dirPath === '/local' || dirPath === '/') {
      entries.push(
        { name: 'local', path: '/local', type: 'directory' },
        { name: 'temp', path: '/temp', type: 'directory' }
      );
    }

    // Add files that exist in this directory
    for (const [filePath, file] of mockFiles.entries()) {
      if (filePath.startsWith(dirPath)) {
        const relativePath = filePath.substring(dirPath.length + 1);
        const parts = relativePath.split('/');

        if (parts.length === 1) {
          // Direct file in this directory
          entries.push({
            name: parts[0],
            path: filePath,
            type: 'file'
          });
        }
      }
    }

    return HttpResponse.json({
      path: dirPath,
      entries
    });
  }),

  // GET /api/v1/stat/:path - Get file/directory stats
  http.get<{ path: string }>('http://localhost:3000/api/v1/stat/:path', () => {
    const filePath = `/${params.path}`;

    if (mockFiles.has(filePath)) {
      const file = mockFiles.get(filePath)!;
      return HttpResponse.json({
        path: filePath,
        type: 'file',
        size: file.content.length,
        created_at: file.created_at,
        updated_at: file.updated_at
      });
    }

    // Check if it's a mount point (directory)
    const mount = mockMounts.find(m => m.path === filePath);
    if (mount) {
      return HttpResponse.json({
        path: filePath,
        type: 'directory',
        size: 0,
        created_at: now(),
        updated_at: now()
      });
    }

    return HttpResponse.json(
      { error: 'Not found', path: filePath },
      { status: 404 }
    );
  })
];

// Plugin handlers
export const pluginHandlers = [
  // GET /api/v1/plugins - List all plugins
  http.get('http://localhost:3000/api/v1/plugins', () => {
    return HttpResponse.json({
      plugins: [
        { name: 'LocalFS', version: '0.1.0', enabled: true },
        { name: 'MemFS', version: '0.1.0', enabled: true },
        { name: 'HTTPFS', version: '0.1.0', enabled: true },
        { name: 'S3FS', version: '0.1.0', enabled: false },
        { name: 'KVFS', version: '0.1.0', enabled: true }
      ]
    });
  })
];

// Health check handlers
export const healthHandlers = [
  http.get('http://localhost:3000/api/v1/health', () => {
    return HttpResponse.json({
      status: 'healthy',
      version: '2.2.0'
    });
  }),

  http.get('http://localhost:3000/health', () => {
    return HttpResponse.json({
      status: 'healthy',
      version: '2.2.0'
    });
  })
];

// Combine all handlers
export const handlers = [
  ...mountHandlers,
  ...fileHandlers,
  ...pluginHandlers,
  ...healthHandlers
];
