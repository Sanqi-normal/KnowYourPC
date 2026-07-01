# KnowYourDisk

> WizTree-like NTFS MFT accelerated disk space analyzer

A Tauri v2 desktop app for ultra-fast disk space analysis on Windows. Reads the NTFS Master File Table directly.

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

### Desktop App

```bash
npm install
npm run tauri build
```

During `tauri dev` or `tauri build`, the MCP binary is automatically compiled and copied to `src-tauri/binaries/` and bundled as a resource in the final installer.

### MCP Server Only (Standalone)

Build just the MCP server binary for standalone use outside the Tauri app:

**Debug:**
```bash
cargo build -p fastscan-mcp
# Binary: target/debug/fastscan-mcp.exe
```

**Release:**
```bash
cargo build -p fastscan-mcp --release
# Binary: target/release/fastscan-mcp.exe
```

The MCP server supports two transport modes:

- **stdio** (default) — communicates over stdin/stdout, intended for MCP hosts that spawn it as a child process
- **HTTP** (`--http`) — runs an Axum HTTP server with SSE transport on `127.0.0.1:3721` (port configurable via `--port`)

### Copy to Tauri Bundle

```bash
npm run build:mcp        # debug build + copy
npm run build:mcp:release # release build + copy
```

Requires Rust 1.77+ and Node.js 18+.

## MCP Server

The bundled MCP server (`fastscan-mcp`) exposes disk analysis tools over the [Model Context Protocol](https://modelcontextprotocol.io), enabling AI coding agents to scan volumes, browse directories, search files, and analyze disk usage directly.

### Tools

| Tool | Description |
|------|-------------|
| `list_volumes` | List all disk volumes with capacity and filesystem info |
| `scan_disk` | Deep scan a disk volume (NTFS MFT ~100x faster, requires admin) |
| `scan_status` | Get current scan result summary |
| `browse_directory` | Get children of a directory node |
| `get_node_path` | Get the full path of a file/directory node |
| `get_node_details` | Get a node plus all its ancestors |
| `search_files` | Search files/folders by name with optional filters |
| `get_extension_stats` | Get file extension statistics from the last scan |
| `get_treemap` | Get treemap visualization data for a directory |
| `get_largest_files` | Get the largest files from the last scan |
| `get_largest_directories` | Get the largest directories from the last scan |
| `find_empty_directories` | Find empty directories |
| `find_duplicate_files` | Find duplicate files by name and size |
| `find_files_by_age` | Find files by modification time |

### Usage

```bash
# stdio mode (default) — for MCP hosts that spawn child processes
fastscan-mcp

# HTTP mode — for remote or SSE-based MCP hosts
fastscan-mcp --http --port 3721
```

## MCP Host Configuration

Below are the configuration examples for connecting various MCP-compatible AI coding agents to the FastScan MCP server. Each host supports two transport methods:

- **stdio** — the MCP host spawns `fastscan-mcp` as a child process (recommended for local use)
- **HTTP/SSE** — run `fastscan-mcp --http` first, then configure the host to connect to its URL

> **Note:** For HTTP mode on Windows, use the full path to the `fastscan-mcp.exe` binary (e.g., `D:\tools\fastscan-mcp.exe`).

---

### VS Code (GitHub Copilot)

**Config file:** `.vscode/mcp.json` (workspace-level, uses `"servers"` key, not `"mcpServers"`)

**stdio:**
```jsonc
{
  "servers": {
    "fastscan": {
      "type": "stdio",
      "command": "D:\\tools\\fastscan-mcp.exe"
    }
  }
}
```

**HTTP:**
```jsonc
{
  "servers": {
    "fastscan": {
      "type": "http",
      "url": "http://127.0.0.1:3721/sse"
    }
  }
}
```

---

### Claude Desktop

**Config file:** `%APPDATA%\Claude\claude_desktop_config.json`

**stdio:**
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "command": "D:\\tools\\fastscan-mcp.exe"
    }
  }
}
```

**HTTP:**
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "transport": "sse",
      "url": "http://127.0.0.1:3721/sse"
    }
  }
}
```

---

### Claude Code (CLI)

**Config file:** `.mcp.json` (project-level) or `~/.claude.json` (user-level)

**stdio:**
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "command": "D:\\tools\\fastscan-mcp.exe"
    }
  }
}
```

**HTTP** (also via CLI command):
```bash
claude mcp add --transport sse fastscan http://127.0.0.1:3721/sse
```
Or in config file:
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "type": "http",
      "url": "http://127.0.0.1:3721/sse"
    }
  }
}
```

---

### OpenCode

**Config file:** `opencode.json` (project root)

**stdio:**
```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "fastscan": {
      "type": "local",
      "command": ["D:\\tools\\fastscan-mcp.exe"],
      "enabled": true
    }
  }
}
```

**HTTP:**
```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "fastscan": {
      "type": "remote",
      "url": "http://127.0.0.1:3721/sse",
      "enabled": true
    }
  }
}
```

---

### Codex CLI (OpenAI)

**Config file:** `~/.codex/config.toml`

**stdio:**
```toml
[mcp_servers.fastscan]
command = "D:\\tools\\fastscan-mcp.exe"
```

**HTTP:**
```toml
[mcp_servers.fastscan]
url = "http://127.0.0.1:3721/sse"
```

Or via CLI:
```bash
codex mcp add fastscan -- D:\\tools\\fastscan-mcp.exe
```

---

### Cursor

**Config file:** `.cursor/mcp.json` (project-level) or `~/.cursor/mcp.json` (user-level)

**stdio:**
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "command": "D:\\tools\\fastscan-mcp.exe"
    }
  }
}
```

**HTTP:**
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "url": "http://127.0.0.1:3721/sse"
    }
  }
}
```

Also configurable via Cursor settings UI (Features > MCP > Add New MCP Server).

---

### Summary of Config File Locations

| Host | Config File | Root Key |
|------|-------------|----------|
| VS Code | `.vscode/mcp.json` | `servers` |
| Claude Desktop | `%APPDATA%\Claude\claude_desktop_config.json` | `mcpServers` |
| Claude Code | `.mcp.json` / `~/.claude.json` | `mcpServers` |
| OpenCode | `opencode.json` | `mcp` |
| Codex CLI | `~/.codex/config.toml` | `[mcp_servers.*]` |
| Cursor | `.cursor/mcp.json` / `~/.cursor/mcp.json` | `mcpServers` |

## License

MIT
