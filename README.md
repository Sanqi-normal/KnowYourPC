# KnowYourDisk

> 仿 WizTree 的 高速磁盘空间分析和可视化，并为主机提供通用MCP


[English](./README.en.md) · 简体中文

## 特性

- **NTFS MFT 直接扫描** — 解析原始 `$MFT` 记录，近乎即时返回结果
- **Walkdir 回退** — 非 NTFS 卷兼容递归扫描模式
- **矩形树图可视化** — 平铺布局画布渲染
- **文件树** — 虚拟滚动的可展开目录浏览器
- **扩展名统计** — 按文件类型的聚合大小分布
- **文件搜索** — 带防抖输入的实时文件名搜索
- **MCP 服务器** — 通过 Model Context Protocol 集成 AI 编程助手
- **自动提权** — 启动时自动请求管理员权限，取消则以非管理员模式运行

## 技术栈

- **前端**: TypeScript + Vite + Vanilla TypeScript
- **后端**: Rust + Tauri v2
- **NTFS 解析**: 纯 Rust，无外部 NTFS 依赖
- **MCP 服务器**: 独立 Axum HTTP SSE 服务

## 开发环境运行

```bash
npm run dev
```

## 构建

### 桌面应用

```bash
npm install
npm run build
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

MCP 服务器默认以 HTTP SSE 模式运行，在 `127.0.0.1:3721` 监听（端口可通过 `--port` 配置）。

> **注意:** 旧版 stdio 传输模式的代码仍然保留在源码中（`server_stdio.rs`），如果有需要通过子进程方式启动 MCP 的场景，可参考下方 Claude Code 的 stdio 配置示例自行启用。


## MCP 服务器

内置的 MCP 服务器（`fastscan-mcp`）通过 [Model Context Protocol](https://modelcontextprotocol.io) 提供磁盘分析工具，使 AI 编程助手能够扫描卷、浏览目录、搜索文件和分析磁盘使用情况。

### 工具列表

| 工具 | 描述 |
|------|------|
| `list_volumes` | 列出所有磁盘卷的容量和文件系统信息 |
| `scan_disk` | 深度扫描磁盘卷（NTFS MFT需管理员权限，如无则自动回退walkdir） |
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
# 默认启动 (HTTP SSE 模式，端口 3721)
fastscan-mcp

# 自定义端口
fastscan-mcp --port 8080
```

启动时会自动弹出 UAC 请求管理员权限。如果取消授权，服务仍会以非管理员模式运行，但 NTFS MFT 快速扫描不可用。

HTTP 模式也可由应用内点击 MCP 服务启动/停止按钮或托盘图标右键点击启动/停止服务生效。

## MCP 主机配置

以下是各 MCP 兼容 AI 编程助手接入 FastScan MCP 服务器的 HTTP SSE 配置示例。先启动 `fastscan-mcp`（默认监听 `127.0.0.1:3721`），再配置主机连接其 URL。




---

### VS Code (GitHub Copilot)

**配置文件:** `.vscode/mcp.json`（工作区级别，使用 `"servers"` 键，不是 `"mcpServers"`）

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

**HTTP**（推荐，也可通过 CLI 命令添加）:
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

**stdio**（旧版模式，需自行启用编译）:
> stdio 传输模式的实现代码已保留在 `server_stdio.rs` 中，如需以子进程方式启动 MCP，可参考此配置自行编译启用（路径需实际需替换为程序实际安装位置）。
```jsonc
{
  "mcpServers": {
    "fastscan": {
      "command": "D:\\KnowYourDisk\\binaries\\fastscan-mcp.exe"
    }
  }
}
```

---

### OpenCode

**配置文件:** `opencode.json`（项目根目录）

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


