import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";
import type {
  NodeDto,
  ProgressEvent,
  ScanMode,
  ScanOptions,
  ScanResult,
  VolumeInfo,
  ExtensionStat,
  ExtCategory,
} from "./types";
import { extCategory, CATEGORY_COLORS, CATEGORY_LABELS } from "./types";
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
let nodes: NodeDto[] = [];
let nodesMap = new Map<number, NodeDto>();
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
const ctxExpandTreemap = qs<HTMLDivElement>("#ctxExpandTreemap");
const ctxOpen = qs<HTMLDivElement>("#ctxOpen");
const ctxCopyPath = qs<HTMLDivElement>("#ctxCopyPath");
const ctxCopyName = qs<HTMLDivElement>("#ctxCopyName");
const splitter1 = qs<HTMLDivElement>("#splitter1");
const splitter2 = qs<HTMLDivElement>("#splitter2");
const treePanel = qs<HTMLElement>("#treePanel");
const extPanel = qs<HTMLElement>("#extPanel");
const adminBanner = qs<HTMLDivElement>("#adminBanner");
const restartAdminBtn = qs<HTMLButtonElement>("#restartAdminBtn");

const treemapTooltip = qs<HTMLDivElement>("#treemapTooltip");
let contextMenuId: number | null = null;

function updateNodesMap() {
  nodesMap.clear();
  for (const n of nodes) {
    nodesMap.set(n.id, n);
  }
}

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

  upButton.addEventListener("click", () => {
    const current = nodes[selectedId];
    if (current?.parent != null) {
      selectNode(current.parent, true, false);
    }
  });

  zoomOutBtn.addEventListener("click", () => {
    const current = nodes[selectedId];
    if (current?.parent != null) {
      selectNode(current.parent, true, true);
    }
  });

  treeViewport.addEventListener("scroll", () => renderRows());

  treeRows.addEventListener("click", (event) => {
    const target = event.target as HTMLElement;
    const row = target.closest<HTMLElement>(".tree-row");
    if (!row) return;
    const id = Number(row.dataset.id);
    if (!Number.isInteger(id) || !nodes[id]) return;
    if (target.closest(".twisty")) {
      toggleExpanded(id);
      return;
    }
    selectNode(id, false, false);
  });

  treeRows.addEventListener("contextmenu", (event) => {
    const target = event.target as HTMLElement;
    const row = target.closest<HTMLElement>(".tree-row");
    if (!row) return;
    const id = Number(row.dataset.id);
    if (!Number.isInteger(id) || !nodes[id]) return;
    event.preventDefault();
    showContextMenu(event.clientX, event.clientY, id);
  });

  treemapCanvas.addEventListener("click", (event) => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const hit = hitTestTreemap(treemapRects, x, y);
    if (hit != null && nodes[hit]) {
      selectNode(hit, true, nodes[hit].isDir);
    }
  });

  treemapCanvas.addEventListener("contextmenu", (event) => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const hit = hitTestTreemapNode(treemapRects, x, y);
    if (hit && nodes[hit.id]) {
      event.preventDefault();
      showContextMenu(event.clientX, event.clientY, hit.id);
    }
  });

  treemapCanvas.addEventListener("mousemove", (event) => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const hit = hitTestTreemapNode(treemapRects, x, y);
    if (hit && hit.id >= 0 && nodes[hit.id]) {
      const path = buildNodePath(hit.id, nodesMap, selectedId);
      treemapTooltip.textContent = `${path}  ${formatBytes(hit.node.totalAllocated)}`;
      treemapTooltip.style.left = `${event.clientX + 12}px`;
      treemapTooltip.style.top = `${event.clientY + 12}px`;
      treemapTooltip.classList.remove("hidden");
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

  ctxExpandTreemap.addEventListener("click", () => {
    if (contextMenuId != null) {
      const node = nodes[contextMenuId];
      if (node && node.isDir) {
        selectNode(contextMenuId, true, true);
      }
    }
    hideContextMenu();
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

async function openInExplorer(nodeId: number) {
  const node = nodes[nodeId];
  if (!node) return;
  const path = nodePath(nodeId);
  try {
    await invoke("open_in_explorer", { path });
  } catch {
    // fallback
  }
}

async function copyNodePath(nodeId: number) {
  const path = nodePath(nodeId);
  try {
    await navigator.clipboard.writeText(path);
  } catch {
    // fallback
  }
}

async function copyNodeName(nodeId: number) {
  const node = nodes[nodeId];
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
    nodes = result.nodes;
    updateNodesMap();
    selectedId = 0;
    expanded = new Set<number>([0]);
    rebuildVisibleRows();
    renderSummary();
    renderWarnings();
    selectNode(0, false, true);
    loadExtensionStats();
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

  const categoryMap = new Map<ExtCategory, { size: number; allocated: number; count: number }>();

  for (const stat of stats) {
    const cat = extCategory(stat.extension);
    const entry = categoryMap.get(cat) ?? { size: 0, allocated: 0, count: 0 };
    entry.size += stat.size;
    entry.allocated += stat.allocated;
    entry.count += stat.fileCount;
    categoryMap.set(cat, entry);
  }

  const sorted = Array.from(categoryMap.entries())
    .sort((a, b) => b[1].allocated - a[1].allocated);

  for (const [cat, data] of sorted) {
    const item = document.createElement("div");
    item.className = "ext-item";

    const pct = (data.allocated / totalAllocated) * 100;
    const color = CATEGORY_COLORS[cat];

    item.innerHTML = `
      <span class="ext-name">${CATEGORY_LABELS[cat]}</span>
      <div class="ext-bar-wrap">
        <div class="ext-bar-fill" style="width:${pct}%;background:${color}"></div>
        <span class="ext-bar-label">${formatBytes(data.allocated)} (${formatNumber(data.count)})</span>
      </div>
      <span class="ext-size">${pct.toFixed(1)}%</span>
    `;

    extList.append(item);
  }
}

function rebuildVisibleRows() {
  if (!nodes.length) {
    visibleRows = [];
    treeSpacer.style.height = "100%";
    return;
  }
  const rows: { id: number; depth: number }[] = [];
  const stack: { id: number; depth: number }[] = [{ id: 0, depth: 0 }];
  while (stack.length > 0) {
    const row = stack.pop()!;
    const node = nodes[row.id];
    if (!node) continue;
    rows.push(row);
    if (expanded.has(row.id) && node.children.length > 0) {
      for (let i = node.children.length - 1; i >= 0; i--) {
        stack.push({ id: node.children[i], depth: row.depth + 1 });
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
    const node = nodes[rowInfo.id];
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

    row.append(
      nameCell,
      numericCell(formatBytes(node.totalAllocated)),
      numericCell(formatBytes(node.totalSize)),
      numericCell(percentOfRoot(node)),
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

function percentOfRoot(node: NodeDto): string {
  const total = result?.totalAllocated ?? 0;
  if (total <= 0) return "—";
  return formatPercent((node.totalAllocated / total) * 100);
}

function toggleExpanded(id: number) {
  if (!nodes[id]) return;
  if (expanded.has(id)) expanded.delete(id);
  else expanded.add(id);
  rebuildVisibleRows();
  renderRows();
}

function selectNode(id: number, scrollIntoView: boolean, expandSelf: boolean) {
  if (!nodes[id]) return;
  selectedId = id;
  ensureAncestorsExpanded(id);
  if (expandSelf && nodes[id].isDir) expanded.add(id);
  rebuildVisibleRows();
  if (scrollIntoView) scrollSelectedIntoView();
  renderRows();
  renderTreemap();
  renderSelectedPath();
}

function ensureAncestorsExpanded(id: number) {
  let current = nodes[id]?.parent;
  while (current != null && nodes[current]) {
    expanded.add(current);
    current = nodes[current].parent;
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

function renderTreemap() {
  treemapRects = drawTreemap(treemapCanvas, nodes, selectedId);
}

function renderSelectedPath() {
  if (!nodes[selectedId]) {
    selectedPath.textContent = "未选择";
    return;
  }
  const node = nodes[selectedId];
  selectedPath.textContent = `${nodePath(selectedId)} ${formatBytes(node.totalAllocated)}`;
}

function nodePath(id: number): string {
  const parts: string[] = [];
  let current: number | null | undefined = id;
  while (current != null && nodes[current]) {
    parts.push(nodes[current].name);
    current = nodes[current].parent;
  }
  parts.reverse();
  if (parts.length === 0) return "";
  let path = parts[0];
  for (let i = 1; i < parts.length; i++) {
    if (path.endsWith("\\") || path.endsWith("/")) path += parts[i];
    else path += `\\${parts[i]}`;
  }
  return path;
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
