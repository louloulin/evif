# EVIF 2.2 - Web UI

Web-based user interface for EVIF (Extensible Virtual Interface Framework) 2.2, providing a VS Code-like experience for graph file system management.

## Features

- **📁 File Explorer**: Browse and manage files through the REST API
- **✏️ Code Editor**: Full-featured Monaco editor with syntax highlighting
- **🖥️ Terminal**: Integrated terminal with WebSocket support
- **📂 Resizable Layout**: Drag to resize sidebar and terminal panels
- **🎨 VS Code Dark Theme**: Familiar dark theme for comfortable editing

## Technology Stack

- **Bun 1.3+**: Fast JavaScript runtime and package manager
- **React 18.2.0**: Modern UI framework
- **TypeScript 5.0+**: Type-safe development
- **Monaco Editor**: VS Code's editor (via @monaco-editor/react)
- **XTerm.js**: Terminal emulator (via @xterm/xterm)
- **react-split**: Resizable split panels

## Prerequisites

- [Bun](https://bun.sh/) 1.3 or later
- EVIF REST server running on `http://localhost:8080`
- WebSocket server running on `ws://localhost:8080/ws`

## Installation

```bash
cd evif-web
bun install
```

## Development

Start the development server with hot reload:

```bash
bun run dev
```

The application will be available at `http://localhost:3000`.

## Build for Production

```bash
bun run build
```

The built files will be in the `build/` directory:
- `main.js` - Bundled application (1.48 MB)
- `main.js.map` - Source map for debugging
- `main.css` - Application styles (7.77 KB)

## Type Checking

Run TypeScript type checking without building:

```bash
bun run typecheck
```

## Preview Production Build

```bash
bun run preview
```

## Project Structure

```
evif-web/
├── src/
│   ├── components/
│   │   ├── ContextMenu.tsx   # Right-click context menu
│   │   ├── Editor.tsx         # Monaco code editor
│   │   ├── FileTree.tsx       # File explorer
│   │   ├── MenuBar.tsx        # Top menu bar
│   │   └── Terminal.tsx       # XTerm terminal
│   ├── App.tsx               # Main application component
│   ├── App.css               # Application styles
│   └── main.tsx              # React entry point
├── build/                    # Production build output
├── index.html                # HTML template
├── tsconfig.json            # TypeScript configuration
├── package.json             # Dependencies and scripts
└── bun.lock                 # Bun lockfile
```

## API Integration

The frontend connects to the EVIF REST API:

- **File Listing**: `GET /api/v1/fs/list?path=/`
- **Read File**: `GET /api/v1/fs/read?path={path}`
- **Write File**: `POST /api/v1/fs/write?path={path}`
- **Create File**: `POST /api/v1/fs/create`
- **Delete File**: `DELETE /api/v1/fs/delete?path={path}`
- **WebSocket**: `ws://localhost:8080/ws` (for terminal commands)

## Keyboard Shortcuts

- **Ctrl/Cmd + S**: Save current file
- **Escape**: Close context menu
- **Terminal**: Enter to execute commands, Backspace to delete

## Architecture

The application follows a VS Code-like three-column layout:

1. **Sidebar** (left): File tree explorer with expandable folders
2. **Editor** (center): Monaco editor with tab bar
3. **Terminal** (bottom): XTerm terminal emulator

All panels are resizable via drag handles.

## Development Benefits of Bun

- **Fast Installation**: `bun install` is significantly faster than npm
- **Quick Builds**: Bun's bundler is optimized for speed
- **TypeScript Support**: Built-in TypeScript compilation
- **Hot Module Replacement**: Instant updates during development
- **Smaller Bundle Size**: Optimized production builds

## Current Limitations

- WebSocket backend support needs implementation
- No file upload/download functionality yet
- Limited terminal command set
- No plugin management UI (planned for EVIF 2.2+)

## Future Enhancements

See `evif2.2.md` for the complete roadmap:

1. Plugin management interface
2. Monitoring dashboard
3. Collaboration features (shared sessions)
4. Advanced search functionality
5. File upload/download
6. Multi-file tabs support

## License

Part of the EVIF project. See main project LICENSE file.
