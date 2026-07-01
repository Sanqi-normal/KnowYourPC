# KnowYourDisk

> WizTree-like NTFS MFT accelerated disk space analyzer

A Tauri v2 desktop app for ultra-fast disk space analysis on Windows. Reads the NTFS Master File Table directly, achieving ~100x speed improvement over traditional recursive directory traversal.

## Features

- **NTFS MFT Direct Scan** — parses raw `$MFT` records for near-instant results
- **Walkdir Fallback** — compatible recursive mode for non-NTFS volumes
- **Treemap Visualization** — squarified layout canvas rendering
- **File Tree** — virtual-scrolled, expandable directory browser
- **Extension Stats** — aggregated size breakdown by file type
- **File Search** — real-time name search with debounced input
- **MCP Server** — bundled AI agent integration via Model Context Protocol
- **Admin Elevation** — UAC restart for NTFS raw device access

## Tech Stack

- **Frontend**: TypeScript + Vite + Vanilla DOM
- **Backend**: Rust + Tauri v2
- **NTFS Parsing**: Pure Rust, zero external NTFS dependencies
- **MCP Server**: Standalone Axum HTTP / stdio JSON-RPC server

## Build

```bash
npm install
npm run tauri build
```

Requires Rust 1.77+ and Node.js 18+.

## License

MIT
