import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";
import type {
  ChildNode,
  ProgressEvent,
  ScanMode,
  ScanOptions,
  ScanResult,
  VolumeInfo,
  ExtensionStat,
  SearchResult,
  TreemapItem,
} from "./types";
import {
  formatBytes,
  formatDuration,
  formatNumber,
  formatPercent
} from "./format";
import {
  drawTreemap,
  hitTestTreemap,
  hitTestTreemapNode,
  buildNodePath,
  type TreemapRect,
} from "./treemap";

const ROW_HEIGHT = 26;
const OVERSCAN_ROWS = 18;

let volumes: VolumeInfo[] = [];
let result: ScanResult | null = null;
let nodeCache = new Map<number, ChildNode>();
let parentChildren = new Map<number, number[]>();
let expanded = new Set<number>([0]);
let selectedId = 0;
let visibleRows: { id: number; depth: number }[] = [];
let treemapRects: TreemapRect[] = [];
let scanning = false;

function qs<T extends HTMLElement>(sel: string): T {
  return document.querySelector<T>(sel)!;
}

const volumeSelect = qs<HTMLSelectElement>("#volumeSelect");
const modeSelect = qs<HTMLSelectElement>("#modeSelect");
const refreshVolumesButton = qs<HTMLButtonElement>("#refreshVolumesButton");
const scanButton = qs<HTMLButtonElement>("#scanButton");
const progressFill = qs<HTMLDivElement>("#progressFill");
const statusText = qs<HTMLDivElement>("#statusText");
const summaryAllocated = qs<HTMLDivElement>("#summaryAllocated");
const summarySize = qs<HTMLDivElement>("#summarySize");
const summaryFiles = qs<HTMLDivElement>("#summaryFiles");
const summaryDirs = qs<HTMLDivElement>("#summaryDirs");
const summaryScanner = qs<HTMLDivElement>("#summaryScanner");
const selectedPath = qs<HTMLDivElement>("#selectedPath");
const upButton = qs<HTMLButtonElement>("#upButton");
const zoomOutBtn = qs<HTMLButtonElement>("#zoomOutBtn");
const treeViewport = qs<HTMLDivElement>("#treeViewport");
const treeSpacer = qs<HTMLDivElement>("#treeSpacer");
const treeRows = qs<HTMLDivElement>("#treeRows");
const treemapCanvas = qs<HTMLCanvasElement>("#treemapCanvas");
const warningsEl = qs<HTMLDivElement>("#warnings");
const extList = qs<HTMLDivElement>("#extList");
const contextMenu = qs<HTMLDivElement>("#contextMenu");
const ctxOpen = qs<HTMLDivElement>("#ctxOpen");
const ctxCopyPath = qs<HTMLDivElement>("#ctxCopyPath");
const ctxCopyName = qs<HTMLDivElement>("#ctxCopyName");
const splitter1 = qs<HTMLDivElement>("#splitter1");
const splitter2 = qs<HTMLDivElement>("#splitter2");
const treePanel = qs<HTMLElement>("#treePanel");
const extPanel = qs<HTMLElement>("#extPanel");
const adminBanner = qs<HTMLDivElement>("#adminBanner");
const restartAdminBtn = qs<HTMLButtonElement>("#restartAdminBtn");

const searchWrap = qs<HTMLDivElement>("#searchWrap");
const searchInput = qs<HTMLInputElement>("#searchInput");
const searchResults = qs<HTMLDivElement>("#searchResults");

const treemapTooltip = qs<HTMLDivElement>("#treemapTooltip");
let contextMenuId: number | null = null;
let searchTimeout: ReturnType<typeof setTimeout> | null = null;

async function bootstrap() {
  wireEvents();
  checkAdmin();
  await listen<ProgressEvent>("scan-progress", (event) => {
    renderProgress(event.payload);
  });
  await loadVolumes();
  renderSummary();
  renderRows();
  renderTreemap();
  initSplitters();
}

async function checkAdmin() {
  try {
    const elevated = await invoke<boolean>("is_admin");
    if (!elevated) {
      adminBanner.classList.remove("hidden");
    }
  } catch {
    // 忽略检查失败
  }
}

function wireEvents() {
  refreshVolumesButton.addEventListener("click", () => loadVolumes());

  scanButton.addEventListener("click", () => startScan());

  upButton.addEventListener("click", async () => {
    const current = nodeCache.get(selectedId);
    if (current?.parent != null) {
      await selectNode(current.parent, true, false);
    }
  });

  zoomOutBtn.addEventListener("click", async () => {
    const current = nodeCache.get(selectedId);
    if (current?.parent != null) {
      await selectNode(current.parent, true, true);
    }
  });

  treeViewport.addEventListener("scroll", () => renderRows());

  treeRows.addEventListener("click", async (event) => {
    const target = event.target as HTMLElement;
    const row = target.closest<HTMLElement>(".tree-row");
    if (!row) return;
    const id = Number(row.dataset.id);
    if (!Number.isInteger(id) || !nodeCache.has(id)) return;
    if (target.closest(".twisty")) {
      await toggleExpanded(id);
      return;
    }
    await selectNode(id, false, false);
  });

  treeRows.addEventListener("mousemove", (event) => {
    const target = event.target as HTMLElement;
    const row = target.closest<HTMLElement>(".tree-row");
    if (!row) {
      treemapTooltip.classList.add("hidden");
      return;
    }
    const id = Number(row.dataset.id);
    if (!Number.isInteger(id) || !nodeCache.has(id)) {
      treemapTooltip.classList.add("hidden");
      return;
    }
    const node = nodeCache.get(id)!;
    const path = buildNodePath(id, nodeCache, 0);
    treemapTooltip.textContent = `${path}  ${formatBytes(node.totalAllocated)}`;
    treemapTooltip.style.left = `${event.clientX + 12}px`;
    treemapTooltip.style.top = `${event.clientY + 12}px`;
    treemapTooltip.classList.remove("hidden");
  });

  treeRows.addEventListener("mouseleave", () => {
    treemapTooltip.classList.add("hidden");
  });

  treeRows.addEventListener("contextmenu", (event) => {
    const target = event.target as HTMLElement;
    const row = target.closest<HTMLElement>(".tree-row");
    if (!row) return;
    const id = Number(row.dataset.id);
    if (!Number.isInteger(id) || !nodeCache.has(id)) return;
    event.preventDefault();
    showContextMenu(event.clientX, event.clientY, id);
  });

  treemapCanvas.addEventListener("click", async (event) => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const hit = hitTestTreemap(treemapRects, x, y);
    if (hit != null) {
      if (!nodeCache.has(hit)) await ensureNodeInCache(hit);
      if (nodeCache.has(hit)) {
        const hitNode = nodeCache.get(hit)!;
        await selectNode(hit, true, hitNode.isDir);
      }
    }
  });

  treemapCanvas.addEventListener("contextmenu", async (event) => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const hit = hitTestTreemapNode(treemapRects, x, y);
    if (hit) {
      if (!nodeCache.has(hit.id)) await ensureNodeInCache(hit.id);
      if (nodeCache.has(hit.id)) {
        event.preventDefault();
        showContextMenu(event.clientX, event.clientY, hit.id);
      }
    }
  });

  treemapCanvas.addEventListener("mousemove", async (event) => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const hit = hitTestTreemapNode(treemapRects, x, y);
    if (hit) {
      if (!nodeCache.has(hit.id)) await ensureNodeInCache(hit.id);
      if (nodeCache.has(hit.id)) {
        const path = buildNodePath(hit.id, nodeCache, selectedId);
        treemapTooltip.textContent = `${path}  ${formatBytes(hit.item.size)}`;
        treemapTooltip.style.left = `${event.clientX + 12}px`;
        treemapTooltip.style.top = `${event.clientY + 12}px`;
        treemapTooltip.classList.remove("hidden");
      } else {
        treemapTooltip.classList.add("hidden");
      }
    } else {
      treemapTooltip.classList.add("hidden");
    }
  });

  treemapCanvas.addEventListener("mouseleave", () => {
    treemapTooltip.classList.add("hidden");
  });

  document.addEventListener("click", () => hideContextMenu());

  ctxOpen.addEventListener("click", () => {
    if (contextMenuId != null) {
      openInExplorer(contextMenuId);
    }
    hideContextMenu();
  });

  ctxCopyPath.addEventListener("click", () => {
    if (contextMenuId != null) {
      copyNodePath(contextMenuId);
    }
    hideContextMenu();
  });

  ctxCopyName.addEventListener("click", () => {
    if (contextMenuId != null) {
      copyNodeName(contextMenuId);
    }
    hideContextMenu();
  });

  searchInput.addEventListener("input", () => {
    if (searchTimeout) clearTimeout(searchTimeout);
    const q = searchInput.value.trim();
    if (q.length < 2) {
      searchResults.classList.add("hidden");
      return;
    }
    searchTimeout = setTimeout(() => doSearch(q), 200);
  });

  searchInput.addEventListener("blur", () => {
    setTimeout(() => searchResults.classList.add("hidden"), 200);
  });

  searchInput.addEventListener("focus", () => {
    if (searchInput.value.trim().length >= 2) {
      searchResults.classList.remove("hidden");
    }
  });

  restartAdminBtn.addEventListener("click", async () => {
    restartAdminBtn.disabled = true;
    restartAdminBtn.textContent = "正在重启...";
    try {
      await invoke("restart_as_admin");
    } catch {
      restartAdminBtn.disabled = false;
      restartAdminBtn.textContent = "以管理员身份重启";
    }
  });

  window.addEventListener("resize", () => {
    renderRows();
    renderTreemap();
  });

  new ResizeObserver(() => renderTreemap()).observe(treemapCanvas);
}

function initSplitters() {
  makeSplitter(splitter1, treePanel, null);
  makeSplitter(splitter2, null, extPanel);
}

function makeSplitter(splitter: HTMLElement, left: HTMLElement | null, right: HTMLElement | null) {
  let dragging = false;
  let startX = 0;
  let startLeft = 0;
  let startRight = 0;

  splitter.addEventListener("mousedown", (e) => {
    dragging = true;
    startX = e.clientX;
    startLeft = left ? left.getBoundingClientRect().width : 0;
    startRight = right ? right.getBoundingClientRect().width : 0;
    splitter.classList.add("active");
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  });

  document.addEventListener("mousemove", (e) => {
    if (!dragging) return;
    const dx = e.clientX - startX;
    if (left) {
      const newW = Math.max(200, Math.min(800, startLeft + dx));
      left.style.flex = `0 0 ${newW}px`;
    }
    if (right) {
      const newW = Math.max(160, Math.min(500, startRight - dx));
      right.style.flex = `0 0 ${newW}px`;
    }
  });

  document.addEventListener("mouseup", () => {
    if (dragging) {
      dragging = false;
      splitter.classList.remove("active");
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    }
  });
}

function showContextMenu(x: number, y: number, id: number) {
  contextMenuId = id;
  contextMenu.style.left = `${x}px`;
  contextMenu.style.top = `${y}px`;
  contextMenu.classList.remove("hidden");
}

function hideContextMenu() {
  contextMenu.classList.add("hidden");
  contextMenuId = null;
}

async function doSearch(query: string) {
  try {
    const items = await invoke<SearchResult[]>("search_files", {
      query,
      maxResults: 100,
    });
    renderSearchResults(items);
  } catch {
    searchResults.classList.add("hidden");
  }
}

function renderSearchResults(items: SearchResult[]) {
  searchResults.replaceChildren();
  if (items.length === 0) {
    const div = document.createElement("div");
    div.className = "search-result-item";
    div.textContent = "未找到匹配项";
    searchResults.append(div);
    searchResults.classList.remove("hidden");
    return;
  }

  for (const item of items.slice(0, 100)) {
    const row = document.createElement("div");
    row.className = "search-result-item";
    row.innerHTML = `
      <span class="search-result-icon">${item.isDir ? "\uD83D\uDCC1" : "\uD83D\uDCC4"}</span>
      <span class="search-result-name">${escapeHtml(item.name)}</span>
      <span class="search-result-size">${formatBytes(item.totalAllocated)}</span>
      <span class="search-result-path">${escapeHtml(shortenPath(item.path))}</span>
    `;
    row.addEventListener("click", async () => {
      searchResults.classList.add("hidden");
      searchInput.blur();
      await selectNode(item.id, true, item.isDir);
    });
    row.addEventListener("mousedown", (e) => e.preventDefault());
    searchResults.append(row);
  }

  searchResults.classList.remove("hidden");
}

function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

function shortenPath(path: string): string {
  if (path.length <= 80) return path;
  return "..." + path.slice(-77);
}

async function openInExplorer(nodeId: number) {
  const node = nodeCache.get(nodeId);
  if (!node) return;
  const path = await nodePath(nodeId);
  try {
    await invoke("open_in_explorer", { path });
  } catch (err) {
    setStatus(`打开失败: ${err}`);
  }
}

async function copyNodePath(nodeId: number) {
  const path = await nodePath(nodeId);
  try {
    await navigator.clipboard.writeText(path);
  } catch {
    // fallback
  }
}

async function copyNodeName(nodeId: number) {
  const node = nodeCache.get(nodeId);
  if (!node) return;
  try {
    await navigator.clipboard.writeText(node.name);
  } catch {
    // fallback
  }
}

async function loadVolumes() {
  try {
    setStatus("枚举磁盘卷...");
    volumes = await invoke<VolumeInfo[]>("list_volumes");
    renderVolumeOptions();
    setStatus("就绪");
  } catch (error) {
    setStatus(`枚举卷失败: ${String(error)}`);
  }
}

function renderVolumeOptions() {
  const previous = volumeSelect.value;
  volumeSelect.replaceChildren();
  for (const volume of volumes) {
    const option = document.createElement("option");
    option.value = volume.root;
    const fs = volume.fsName ? ` ${volume.fsName}` : "";
    const total = volume.totalBytes > 0 ? ` ${formatBytes(volume.totalBytes)}` : "";
    option.textContent = `${volume.displayName}${fs}${total}`;
    volumeSelect.append(option);
  }
  if (previous && volumes.some((v) => v.root === previous)) {
    volumeSelect.value = previous;
    return;
  }
  const preferred = volumes.find((v) => v.ntfsCandidate && v.driveType === "Fixed")
    ?? volumes.find((v) => v.ntfsCandidate)
    ?? volumes[0];
  if (preferred) volumeSelect.value = preferred.root;
}

async function loadChildren(parentId: number): Promise<ChildNode[]> {
  const children = await invoke<ChildNode[]>("get_children", { parentId });
  const childIds: number[] = [];
  for (const child of children) {
    nodeCache.set(child.id, child);
    childIds.push(child.id);
  }
  parentChildren.set(parentId, childIds);
  return children;
}

async function ensureNodeInCache(id: number) {
  if (nodeCache.has(id)) return;
  const ancestors = await invoke<ChildNode[]>("get_node_with_ancestors", { nodeId: id });
  for (const node of ancestors) {
    if (!nodeCache.has(node.id)) {
      nodeCache.set(node.id, node);
    }
  }
}

async function ensureTreemapNodesInCache(rects: TreemapRect[]) {
  const missing = new Set<number>();
  for (const rect of rects) {
    if (!nodeCache.has(rect.id)) missing.add(rect.id);
  }
  if (missing.size === 0) return;
  const results = await Promise.all(
    [...missing].map((id) =>
      invoke<ChildNode[]>("get_node_with_ancestors", { nodeId: id }).catch(() => [] as ChildNode[])
    )
  );
  for (const nodes of results) {
    for (const node of nodes) {
      if (!nodeCache.has(node.id)) {
        nodeCache.set(node.id, node);
      }
    }
  }
}

async function startScan() {
  if (scanning) return;
  const root = volumeSelect.value;
  if (!root) { setStatus("没有可扫描的卷"); return; }

  const options: ScanOptions = {
    root,
    mode: modeSelect.value as ScanMode,
    includeSystemFiles: true,
  };

  scanning = true;
  scanButton.disabled = true;
  refreshVolumesButton.disabled = true;
  progressFill.style.width = "0%";
  setStatus("准备扫描...");
  extList.replaceChildren();

  try {
    result = await invoke<ScanResult>("scan", { options });
    nodeCache.clear();
    parentChildren.clear();
    selectedId = 0;
    expanded = new Set<number>([0]);
    const rootChildren = await loadChildren(0);
    nodeCache.set(0, {
      id: 0,
      parent: null,
      name: result.root,
      isDir: true,
      size: 0,
      allocated: 0,
      totalSize: result.totalSize,
      totalAllocated: result.totalAllocated,
      childCount: rootChildren.length,
      fileCount: result.fileCount,
      dirCount: result.dirCount,
      extension: null,
    });
    rebuildVisibleRows();
    renderSummary();
    renderWarnings();
    await selectNode(0, false, true);
    loadExtensionStats();
    searchWrap.classList.remove("hidden");
    searchInput.value = "";
    searchResults.classList.add("hidden");
    progressFill.style.width = "100%";
    setStatus(`扫描完成 (${formatDuration(result.elapsedMs)})`);
  } catch (error) {
    setStatus(`扫描失败: ${String(error)}`);
  } finally {
    scanning = false;
    scanButton.disabled = false;
    refreshVolumesButton.disabled = false;
  }
}

function renderProgress(progress: ProgressEvent) {
  if (progress.total && progress.total > 0) {
    progressFill.style.width = `${Math.min(100, (progress.processed / progress.total) * 100)}%`;
  }
  setStatus(progress.message);
}

function renderSummary() {
  if (!result) {
    summaryAllocated.textContent = "—";
    summarySize.textContent = "—";
    summaryFiles.textContent = "—";
    summaryDirs.textContent = "—";
    summaryScanner.textContent = "—";
    return;
  }
  summaryAllocated.textContent = formatBytes(result.totalAllocated);
  summarySize.textContent = formatBytes(result.totalSize);
  summaryFiles.textContent = formatNumber(result.fileCount);
  summaryDirs.textContent = formatNumber(result.dirCount);
  summaryScanner.textContent = `${result.scanner} ${formatDuration(result.elapsedMs)}`;
}

async function loadExtensionStats() {
  try {
    const stats = await invoke<ExtensionStat[]>("get_extension_stats");
    renderExtensionStats(stats);
  } catch {
    // ignore
  }
}

function renderExtensionStats(stats: ExtensionStat[]) {
  extList.replaceChildren();
  if (stats.length === 0) return;

  const totalAllocated = stats.reduce((s, st) => s + st.allocated, 0);
  if (totalAllocated <= 0) return;

  for (const stat of stats) {
    const item = document.createElement("div");
    item.className = "ext-item";

    const pct = (stat.allocated / totalAllocated) * 100;
    const ext = stat.extension || "(无)";
    const hue = hashExt(ext);

    item.innerHTML = `
      <span class="ext-name">.${ext}</span>
      <div class="ext-bar-wrap">
        <div class="ext-bar-fill" style="width:${pct}%;background:hsl(${hue},55%,50%)"></div>
        <span class="ext-bar-label">${formatBytes(stat.allocated)} (${formatNumber(stat.fileCount)})</span>
      </div>
      <span class="ext-size">${pct.toFixed(1)}%</span>
    `;

    extList.append(item);
  }
}

function hashExt(text: string): number {
  let hash = 2166136261;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash) % 360;
}

function rebuildVisibleRows() {
  const rows: { id: number; depth: number }[] = [];
  const stack: { id: number; depth: number }[] = [{ id: 0, depth: 0 }];
  while (stack.length > 0) {
    const row = stack.pop()!;
    const node = nodeCache.get(row.id);
    if (!node) continue;
    rows.push(row);
    if (expanded.has(row.id) && node.childCount > 0) {
      const childIds = parentChildren.get(row.id);
      if (childIds) {
        for (let i = childIds.length - 1; i >= 0; i--) {
          stack.push({ id: childIds[i], depth: row.depth + 1 });
        }
      }
    }
  }
  visibleRows = rows;
  treeSpacer.style.height = `${Math.max(1, visibleRows.length * ROW_HEIGHT)}px`;
}

function renderRows() {
  if (!visibleRows.length) {
    treeRows.replaceChildren();
    return;
  }
  const scrollTop = treeViewport.scrollTop;
  const viewportHeight = treeViewport.clientHeight;
  const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN_ROWS);
  const end = Math.min(visibleRows.length, Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN_ROWS);
  const fragment = document.createDocumentFragment();

  for (let index = start; index < end; index++) {
    const rowInfo = visibleRows[index];
    const node = nodeCache.get(rowInfo.id);
    if (!node) continue;

    const row = document.createElement("div");
    row.className = `tree-row${node.id === selectedId ? " selected" : ""}`;
    row.style.top = `${index * ROW_HEIGHT}px`;
    row.dataset.id = String(node.id);

    const nameCell = document.createElement("div");
    nameCell.className = "cell name-cell";
    nameCell.style.paddingLeft = `${8 + rowInfo.depth * 14}px`;

    const twisty = document.createElement("button");
    twisty.type = "button";
    twisty.className = "twisty";
    twisty.disabled = node.childCount === 0;
    twisty.textContent = node.childCount === 0 ? "" : expanded.has(node.id) ? "▾" : "▸";

    const icon = document.createElement("span");
    icon.className = "file-icon";
    icon.textContent = node.isDir ? "\uD83D\uDCC1" : "\uD83D\uDCC4";

    const nameText = document.createElement("span");
    nameText.textContent = node.name;

    nameCell.append(twisty, icon, nameText);

    const parentNode = node.parent != null ? nodeCache.get(node.parent) : null;
    const parentTotal = parentNode ? parentNode.totalAllocated : result?.totalAllocated;
    const pct = parentTotal ? (node.totalAllocated / parentTotal) * 100 : 0;
    const pctCell = document.createElement("div");
    pctCell.className = "cell pct-cell";
    pctCell.innerHTML = `<div class="pct-bar"><div class="pct-bar-fill" style="width:${pct}%"></div><span class="pct-bar-label">${formatPercent(pct)}</span></div>`;

    row.append(
      nameCell,
      numericCell(formatBytes(node.totalAllocated)),
      pctCell,
      numericCell(formatNumber(node.fileCount + node.dirCount))
    );

    fragment.append(row);
  }

  treeRows.replaceChildren(fragment);
}

function numericCell(text: string): HTMLDivElement {
  const cell = document.createElement("div");
  cell.className = "cell numeric";
  cell.textContent = text;
  return cell;
}

async function toggleExpanded(id: number) {
  const node = nodeCache.get(id);
  if (!node) return;
  if (expanded.has(id)) {
    expanded.delete(id);
  } else {
    if (node.childCount > 0) {
      await loadChildren(id);
    }
    expanded.add(id);
  }
  rebuildVisibleRows();
  renderRows();
}

async function selectNode(id: number, scrollIntoView: boolean, expandSelf: boolean) {
  const node = nodeCache.get(id);
  if (!node) return;
  selectedId = id;
  await ensureAncestorsExpanded(id);
  if (expandSelf && node.isDir) {
    if (node.childCount > 0) {
      await loadChildren(id);
    }
    expanded.add(id);
  }
  rebuildVisibleRows();
  if (scrollIntoView) scrollSelectedIntoView();
  renderRows();
  await renderTreemap();
  await renderSelectedPath();
}

async function ensureAncestorsExpanded(id: number) {
  const path: number[] = [];
  let current = nodeCache.get(id)?.parent;
  while (current != null && nodeCache.has(current)) {
    path.push(current);
    current = nodeCache.get(current)?.parent;
  }
  for (let i = path.length - 1; i >= 0; i--) {
    const pid = path[i];
    if (!expanded.has(pid)) {
      await loadChildren(pid);
      expanded.add(pid);
    }
  }
}

function scrollSelectedIntoView() {
  const index = visibleRows.findIndex((row) => row.id === selectedId);
  if (index < 0) return;
  const rowTop = index * ROW_HEIGHT;
  const rowBottom = rowTop + ROW_HEIGHT;
  if (rowTop < treeViewport.scrollTop) {
    treeViewport.scrollTop = rowTop;
  } else if (rowBottom > treeViewport.scrollTop + treeViewport.clientHeight) {
    treeViewport.scrollTop = rowBottom - treeViewport.clientHeight;
  }
}

async function renderTreemap() {
  if (!nodeCache.has(selectedId)) return;
  try {
    const items = await invoke<TreemapItem[]>("get_treemap_data", {
      rootId: selectedId,
      maxItems: 3000,
    });
    treemapRects = drawTreemap(treemapCanvas, items);
    ensureTreemapNodesInCache(treemapRects);
  } catch {
    treemapRects = [];
  }
}

async function renderSelectedPath() {
  const node = nodeCache.get(selectedId);
  if (!node) {
    selectedPath.textContent = "未选择";
    return;
  }
  const path = await nodePath(selectedId);
  selectedPath.textContent = `${path} ${formatBytes(node.totalAllocated)}`;
}

async function nodePath(id: number): Promise<string> {
  if (!nodeCache.has(id)) return "";
  return await invoke<string>("get_node_path", { nodeId: id });
}

function renderWarnings() {
  warningsEl.replaceChildren();
  if (!result?.warnings.length) return;
  for (const warning of result.warnings) {
    const div = document.createElement("div");
    div.className = "warning-item";
    div.textContent = warning;
    warningsEl.append(div);
  }
}

function setStatus(message: string) {
  statusText.textContent = message;
}

void bootstrap();
