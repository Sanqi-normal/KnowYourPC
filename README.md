# KnowYourPC

> 快速了解你的电脑 —— 为人类用户和 AI Agent 提供深度系统洞察的工具

## 概述

KnowYourPC 是一款基于 **Tauri v2** 的桌面工具，旨在以极快的速度扫描和分析 Windows 电脑的磁盘空间使用情况。它直接读取 NTFS 卷的 **MFT（Master File Table）**，绕过了传统文件系统遍历 API，实现了接近 WizTree 级别的扫描速度。

## 当前实现功能

### 核心技术

| 模块 | 描述 |
|------|------|
| **NTFS MFT 直读** | 绕过 `FindFirstFile`/`FindNextFile`，直接通过 `\\.\C:` 打开卷句柄，顺序读取并解析 `$MFT` 的全部 FILE 记录 |
| **Boot Sector 解析** | 读取 NTFS 引导扇区，提取每簇字节数、MFT 起始 LCN、FILE 记录大小等几何参数 |
| **Update Sequence Array 修正** | 对每条 MFT 记录执行 USA fixup，确保数据完整性 |
| **Data Run 解析** | 解析非常驻属性的 mapping pairs，支持碎片的 `$MFT` 分布定位 |
| **目录树重建** | 通过 `$FILE_NAME` 属性的 Parent FRN 重建完整目录层次结构，自底向上聚合大小 |
| **双扫描模式** | NTFS MFT 模式（高速）+ walkdir 回退模式（兼容），Auto 模式自动选择 |
| **进度事件推送** | 扫描过程中通过 Tauri event 实时推送进度到前端 |

### 前端可视化

| 组件 | 描述 |
|------|------|
| **卷选择器** | 枚举 Windows 所有卷，显示文件系统类型、总容量，标记 NTFS 候选卷 |
| **扫描模式切换** | Auto / NTFS MFT / 兼容递归 三种模式 |
| **概览仪表盘** | 占用空间、逻辑大小、文件数、目录数、扫描器信息 |
| **文件树面板** | 虚拟滚动（overscan）的目录树，支持展开/折叠、点击定位、父级导航 |
| **Treemap 矩形树图** | Canvas 渲染的 slice-dice 分层 treemap，点击色块可定位节点 |
| **路径显示** | 当前选中节点的完整路径展示 |
| **警告栏** | 扫描过程中的错误/跳过信息汇总 |

### 技术栈

- **前端**: TypeScript + Vite 6 + Vanilla DOM API
- **后端**: Rust + Tauri v2
- **NTFS 解析**: 纯 Rust 实现，零外部 NTFS 依赖
- **回退扫描**: `walkdir` crate
- **卷信息**: Windows API (`GetVolumeInformationW`, `GetDiskFreeSpaceExW`, `GetDriveTypeW`)

### 项目结构

```
KnowYourPC/
├── index.html                  # 主页面 HTML
├── package.json                # Node 依赖
├── tsconfig.json               # TypeScript 配置
├── vite.config.ts              # Vite 构建配置
├── src/                        # 前端源码
│   ├── main.ts                 # 应用主逻辑
│   ├── styles.css              # 样式
│   ├── types.ts                # 类型定义
│   ├── format.ts               # 格式化工具
│   └── treemap.ts              # Treemap 可视化（slice-dice 布局）
└── src-tauri/                  # 后端源码
    ├── Cargo.toml              # Rust 依赖
    ├── build.rs                # 构建脚本
    ├── tauri.conf.json         # Tauri 配置
    ├── capabilities/default.json
    └── src/
        ├── main.rs             # 入口
        ├── lib.rs              # Tauri Builder 装配
        ├── commands.rs         # Tauri 命令（list_volumes, scan）
        ├── models.rs           # 数据模型
        ├── win/
        │   └── volume.rs       # Windows 卷枚举
        └── scanner/
            ├── mod.rs          # 扫描入口与调度
            ├── tree.rs         # 目录树重建与累加
            ├── path_walk.rs    # 兼容递归扫描
            └── ntfs/
                ├── mod.rs      # NTFS 扫描编排
                ├── boot.rs     # 引导扇区解析
                ├── record.rs   # MFT 记录解析 + attribute 解析
                └── runs.rs     # Data run / mapping pairs 解析
```

### 数据流

```
用户点击"开始扫描"
    → 前端调用 invoke("scan", { root, mode })
    → Rust 后端根据模式选择扫描引擎
        → NTFS MFT 模式:
            1. CreateFileW 打开 \\.\C:
            2. 读取 Boot Sector → 获取卷几何参数
            3. 定位 $MFT 记录 0 → 解析 DATA 属性的 data runs
            4. 按 data run 顺序流式读取全部 MFT 记录
            5. 对每条记录做 USA fixup → 解析 FILE_NAME / DATA 属性
            6. 通过 Parent FRN 重建目录树 → 自底向上聚合大小
        → 兼容递归模式:
            1. walkdir 遍历整个目录树
            2. 收集文件大小和元数据
            3. 构建目录树并聚合大小
    → 返回 ScanResult 到前端
    → 前端渲染文件树 + Treemap + 概览
```

## 构建与运行

### 前置要求

- Rust 1.77+
- Node.js 18+
- Windows 系统（NTFS MFT 模式仅支持 Windows）

### 开发

```bash
npm install
npm run dev
```

### 构建

```bash
npm run build
```

## 注意事项

- **管理员权限**: NTFS MFT 直读模式需要管理员权限（通过 `\\.\C:` 打开卷句柄），当前版本需右键"以管理员身份运行"
- **仅 Windows**: MFT 直读功能仅适用于 Windows NTFS 卷，非 Windows 平台自动使用兼容递归模式
- **系统文件**: 扫描包含系统文件和隐藏文件

## 已知局限

- 暂未嵌入管理员权限 manifest（需手动以管理员身份运行）
- Treemap 使用 slice-dice 布局，会产生较多长条形色块
- Treemap 颜色基于文件名 hash，未按扩展名分类
- 递归扫描模式为单线程，大目录树速度不够理想
- 无文件右键菜单功能
- 面板不可拖动调整大小
