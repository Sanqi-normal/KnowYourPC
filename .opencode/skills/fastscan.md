---
name: fastscan
description: NTFS MFT-accelerated disk space analyzer. Use when user wants to find what's consuming disk space, locate large files/directories, search files by name, or analyze file type distribution.
mcp_servers:
  - name: fastscan
    command: fastscan-mcp
    args: []
---

## When to Use
- User asks about disk space ("what's taking up space?", "why is my disk full?")
- User wants to find large files or folders
- User searches for files matching a name/pattern
- User wants to see file type (extension) distribution

## Available Tools
| Tool | Description | Input |
|------|-------------|-------|
| `list_volumes` | List all disk volumes with capacity and filesystem info | `{}` |
| `scan_disk` | Deep scan a volume (NTFS MFT ~100x faster, needs admin) | `{root, mode?, includeSystemFiles?}` |
| `browse_directory` | Get children of a directory node | `{parentId}` |
| `get_node_path` | Get the full path of a node | `{nodeId}` |
| `get_node_details` | Get node + all ancestors (breadcrumb) | `{nodeId}` |
| `search_files` | Search files by name (case-insensitive) | `{query, maxResults?}` |
| `get_extension_stats` | Get extension statistics from last scan | `{}` |
| `get_treemap` | Get treemap visualization data | `{rootId, maxItems?}` |

## Recommended Workflow
1. Call `list_volumes` to identify the target drive
2. Call `scan_disk` on the target drive (may take 10-60 seconds)
3. Based on user intent:
   - "What's taking up space?" → `browse_directory` from root, drill down into large items
   - "Find large PDFs" → `search_files` with query, then `get_node_details`
   - "What file types are biggest?" → `get_extension_stats`

## Notes
- NTFS MFT mode is ~100x faster but requires admin privileges
- Scan results are cached in memory until next `scan_disk` call
- Use `get_node_path` to translate node IDs to human-readable paths
