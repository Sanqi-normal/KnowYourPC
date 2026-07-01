# KnowYourDisk

> 仿 WizTree 的 NTFS MFT 加速磁盘空间分析器


[English](./README.en.md) · 简体中文

## 特性

- **NTFS MFT 直接扫描** — 解析原始 `$MFT` 记录，近乎即时返回结果
- **Walkdir 回退** — 非 NTFS 卷兼容递归扫描模式
- **矩形树图可视化** — 平铺布局画布渲染
- **文件树** — 虚拟滚动的可展开目录浏览器
- **扩展名统计** — 按文件类型的聚合大小分布
- **文件搜索** — 带防抖输入的实时文件名搜索
- **MCP 服务器** — 通过 Model Context Protocol 集成 AI 编程助手
- **管理员提权** — UAC 重启获取 NTFS 原始设备访问权限

## 技术栈

- **前端**: TypeScript + Vite + Vanilla DOM
- **后端**: Rust + Tauri v2
- **NTFS 解析**: 纯 Rust，无外部 NTFS 依赖
- **MCP 服务器**: 独立 Axum HTTP / stdio JSON-RPC 服务

## 开发环境运行

```bash
npm run dev
```

## 构建

### 桌面应用

```bash
npm install
npm run  build
```

在 `dev` 或 `build` 过程中，MCP 二进制文件会自动编译并复制到 `src-tauri/binaries/`，最终打包到安装程序中。

### 仅构建 MCP 服务器（独立）

构建独立的 MCP 服务器二进制文件，用于在 Tauri 应用之外使用：

**Debug:**
```bash
cargo build -p fastscan-mcp
# 输出: target/debug/fastscan-mcp.exe
```

**Release:**
```bash
cargo build -p fastscan-mcp --release
# 输出: target/release/fastscan-mcp.exe
```

MCP 服务器支持两种传输模式：

- **stdio**（默认）— 通过标准输入/输出通信，供 MCP 主机作为子进程启动
- **HTTP**（`--http`）— 启动 Axum HTTP 服务器，在 `127.0.0.1:3721` 提供 SSE 传输（端口可通过 `--port` 配置）


## MCP 服务器

内置的 MCP 服务器（`fastscan-mcp`）通过 [Model Context Protocol](https://modelcontextprotocol.io) 提供磁盘分析工具，使 AI 编程助手能够扫描卷、浏览目录、搜索文件和分析磁盘使用情况。

### 工具列表

| 工具 | 描述 |
|------|------|
| `list_volumes` | 列出所有磁盘卷的容量和文件系统信息 |
| `scan_disk` | 深度扫描磁盘卷（NTFS MFT 快约 100 倍，需管理员权限） |
| `scan_status` | 获取当前扫描结果摘要 |
| `browse_directory` | 获取目录节点的子项 |
| `get_node_path` | 获取文件/目录节点的完整路径 |
| `get_node_details` | 获取节点及其所有祖先节点（面包屑导航） |
| `search_files` | 按名称搜索文件/目录（支持可选过滤条件） |
| `get_extension_stats` | 获取上次扫描的文件扩展名统计 |
| `get_treemap` | 获取目录的矩形树图可视化数据 |
| `get_largest_files` | 获取上次扫描中的最大文件 |
| `get_largest_directories` | 获取上次扫描中的最大目录 |
| `find_empty_directories` | 查找空目录 |
| `find_duplicate_files` | 按名称和大小查找重复文件 |
| `find_files_by_age` | 按修改时间查找文件 |

### 使用方法

```bash
# stdio 模式（默认）— 适用于启动子进程的 MCP 主机
fastscan-mcp

# HTTP 模式 — 适用于远程或基于 SSE 的 MCP 主机
# HTTP 模式也可由应用内点击MCP服务启动/停止按钮或托盘图标右键点击启动/停止服务生效
fastscan-mcp --http --port 3721
```

## MCP 主机配置

以下是各 MCP 兼容 AI 编程助手接入 FastScan MCP 服务器的配置示例。每种主机支持两种传输方式：

- **stdio** — MCP 主机将 `fastscan-mcp` 作为子进程启动（推荐本地使用）
- **HTTP/SSE** — 先运行 `fastscan-mcp --http`，再配置主机连接其 URL

> **注意:** Windows 上 HTTP 模式请使用 `fastscan-mcp.exe` 的完整路径（如 `D:\KnowYourDisk\binaries\fastscan-mcp.exe`）。

**以下路径示例均使用`D:\KnowYourDisk\binaries\fastscan-mcp.exe`此示例路径，实际需替换为程序实际安装位置**

---

### VS Code (GitHub Copilot)

**配置文件:** `.vscode/mcp.json`（工作区级别，使用 `"servers"` 键，不是 `"mcpServers"`）

**stdio:**
```jsonc
{
  "servers": {
    "fastscan": {
      "type": "stdio",
      "command": "D:\\KnowYourDisk\\binaries\\fastscan-mcp.exe"
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

**配置文件:** `%APPDATA%\Claude\claude_desktop_config.json`

**stdio:**
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "command": "D:\\KnowYourDisk\\binaries\\fastscan-mcp.exe"
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

### Claude Code（命令行）

**配置文件:** `.mcp.json`（项目级别）或 `~/.claude.json`（用户级别）

**stdio:**
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "command": "D:\\KnowYourDisk\\binaries\\fastscan-mcp.exe"
    }
  }
}
```

**HTTP**（也可通过 CLI 命令添加）:
```bash
claude mcp add --transport sse fastscan http://127.0.0.1:3721/sse
```
或在配置文件中:
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

**配置文件:** `opencode.json`（项目根目录）

**stdio:**
```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "fastscan": {
      "type": "local",
      "command": ["D:\\KnowYourDisk\\binaries\\fastscan-mcp.exe"],
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

**配置文件:** `~/.codex/config.toml`

**stdio:**
```toml
[mcp_servers.fastscan]
command = "D:\\KnowYourDisk\\binaries\\fastscan-mcp.exe"
```

**HTTP:**
```toml
[mcp_servers.fastscan]
url = "http://127.0.0.1:3721/sse"
```

或通过 CLI:
```bash
codex mcp add fastscan -- D:\\KnowYourDisk\\binaries\\fastscan-mcp.exe
```

---

### Cursor

**配置文件:** `.cursor/mcp.json`（项目级别）或 `~/.cursor/mcp.json`（用户级别）

**stdio:**
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "command": "D:\\KnowYourDisk\\binaries\\fastscan-mcp.exe"
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

也可通过 Cursor 设置界面配置（ Features > MCP > Add New MCP Server ）。

---

### 配置文件位置汇总

| 主机 | 配置文件 | 根键 |
|------|----------|------|
| VS Code | `.vscode/mcp.json` | `servers` |
| Claude Desktop | `%APPDATA%\Claude\claude_desktop_config.json` | `mcpServers` |
| Claude Code | `.mcp.json` / `~/.claude.json` | `mcpServers` |
| OpenCode | `opencode.json` | `mcp` |
| Codex CLI | `~/.codex/config.toml` | `[mcp_servers.*]` |
| Cursor | `.cursor/mcp.json` / `~/.cursor/mcp.json` | `mcpServers` |


