import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";
import type {
  NodeDto,
  ProgressEvent,
  ScanMode,
  ScanOptions,
  ScanResult,
  VolumeInfo
} from "./types";
import {
  formatBytes,
  formatDuration,
  formatNumber,
  formatPercent
} from "./format";
import { drawTreemap, hitTestTreemap, type TreemapRect } from "./treemap";

const ROW_HEIGHT = 26;
const OVERSCAN_ROWS = 18;

const volumeSelect = document.querySelector<HTMLSelectElement>("#volumeSelect")!;
const modeSelect = document.querySelector<HTMLSelectElement>("#modeSelect")!;
const refreshVolumesButton = document.querySelector<HTMLButtonElement>(
  "#refreshVolumesButton"
)!;
const scanButton = document.querySelector<HTMLButtonElement>("#scanButton")!;
const progressFill = document.querySelector<HTMLDivElement>("#progressFill")!;
const statusText = document.querySelector<HTMLDivElement>("#statusText")!;

const summaryAllocated =
  document.querySelector<HTMLDivElement>("#summaryAllocated")!;
const summarySize = document.querySelector<HTMLDivElement>("#summarySize")!;
const summaryFiles = document.querySelector<HTMLDivElement>("#summaryFiles")!;
const summaryDirs = document.querySelector<HTMLDivElement>("#summaryDirs")!;
const summaryScanner =
  document.querySelector<HTMLDivElement>("#summaryScanner")!;

const selectedPath = document.querySelector<HTMLDivElement>("#selectedPath")!;
const upButton = document.querySelector<HTMLButtonElement>("#upButton")!;
const treeViewport =
  document.querySelector<HTMLDivElement>("#treeViewport")!;
const treeSpacer = document.querySelector<HTMLDivElement>("#treeSpacer")!;
const treeRows = document.querySelector<HTMLDivElement>("#treeRows")!;
const treemapCanvas =
  document.querySelector<HTMLCanvasElement>("#treemapCanvas")!;
const warningsEl = document.querySelector<HTMLDivElement>("#warnings")!;

interface VisibleRow {
  id: number;
  depth: number;
}

let volumes: VolumeInfo[] = [];
let result: ScanResult | null = null;
let nodes: NodeDto[] = [];
let expanded = new Set<number>([0]);
let selectedId = 0;
let visibleRows: VisibleRow[] = [];
let treemapRects: TreemapRect[] = [];
let scanning = false;

async function bootstrap(): Promise<void> {
  wireEvents();

  await listen<ProgressEvent>("scan-progress", (event) => {
    renderProgress(event.payload);
  });

  await loadVolumes();
  renderSummary();
  renderRows();
  renderTreemap();
}

function wireEvents(): void {
  refreshVolumesButton.addEventListener("click", () => {
    void loadVolumes();
  });

  scanButton.addEventListener("click", () => {
    void startScan();
  });

  upButton.addEventListener("click", () => {
    const current = nodes[selectedId];
    if (current?.parent != null) {
      selectNode(current.parent, true, false);
    }
  });

  treeViewport.addEventListener("scroll", () => {
    renderRows();
  });

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

  treemapCanvas.addEventListener("click", (event) => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const hit = hitTestTreemap(treemapRects, x, y);

    if (hit != null && nodes[hit]) {
      selectNode(hit, true, nodes[hit].isDir);
    }
  });

  window.addEventListener("resize", () => {
    renderRows();
    renderTreemap();
  });

  new ResizeObserver(() => renderTreemap()).observe(treemapCanvas);
}

async function loadVolumes(): Promise<void> {
  try {
    setStatus("正在枚举磁盘卷……");
    volumes = await invoke<VolumeInfo[]>("list_volumes");
    renderVolumeOptions();
    setStatus("准备就绪");
  } catch (error) {
    setStatus(`枚举卷失败：${String(error)}`);
  }
}

function renderVolumeOptions(): void {
  const previous = volumeSelect.value;
  volumeSelect.replaceChildren();

  for (const volume of volumes) {
    const option = document.createElement("option");
    option.value = volume.root;

    const fs = volume.fsName ? ` · ${volume.fsName}` : "";
    const total = volume.totalBytes > 0 ? ` · ${formatBytes(volume.totalBytes)}` : "";
    const mark = volume.ntfsCandidate ? "⚡ " : "";
    option.textContent = `${mark}${volume.displayName}${fs}${total}`;

    volumeSelect.append(option);
  }

  if (previous && volumes.some((volume) => volume.root === previous)) {
    volumeSelect.value = previous;
    return;
  }

  const preferred =
    volumes.find((volume) => volume.ntfsCandidate && volume.driveType === "Fixed") ??
    volumes.find((volume) => volume.ntfsCandidate) ??
    volumes[0];

  if (preferred) {
    volumeSelect.value = preferred.root;
  }
}

async function startScan(): Promise<void> {
  if (scanning) return;

  const root = volumeSelect.value;
  if (!root) {
    setStatus("没有可扫描的卷");
    return;
  }

  const options: ScanOptions = {
    root,
    mode: modeSelect.value as ScanMode,
    includeSystemFiles: true
  };

  scanning = true;
  scanButton.disabled = true;
  refreshVolumesButton.disabled = true;
  progressFill.style.width = "0%";
  setStatus("准备扫描……");

  try {
    result = await invoke<ScanResult>("scan", { options });
    nodes = result.nodes;
    selectedId = 0;
    expanded = new Set<number>([0]);

    rebuildVisibleRows();
    renderSummary();
    renderWarnings();
    selectNode(0, false, true);

    progressFill.style.width = "100%";
    setStatus(`扫描完成，用时 ${formatDuration(result.elapsedMs)}`);
  } catch (error) {
    setStatus(`扫描失败：${String(error)}`);
  } finally {
    scanning = false;
    scanButton.disabled = false;
    refreshVolumesButton.disabled = false;
  }
}

function renderProgress(progress: ProgressEvent): void {
  if (progress.total && progress.total > 0) {
    const percent = Math.min(100, (progress.processed / progress.total) * 100);
    progressFill.style.width = `${percent}%`;
  }

  setStatus(progress.message);
}

function renderSummary(): void {
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
  summaryScanner.textContent = `${result.scanner} · ${formatDuration(
    result.elapsedMs
  )} · ${formatNumber(result.nodeCount)} nodes`;
}

function rebuildVisibleRows(): void {
  if (!nodes.length) {
    visibleRows = [];
    treeSpacer.style.height = "100%";
    return;
  }

  const rows: VisibleRow[] = [];
  const stack: VisibleRow[] = [{ id: 0, depth: 0 }];

  while (stack.length > 0) {
    const row = stack.pop()!;
    const node = nodes[row.id];
    if (!node) continue;

    rows.push(row);

    if (expanded.has(row.id) && node.children.length > 0) {
      for (let i = node.children.length - 1; i >= 0; i -= 1) {
        stack.push({ id: node.children[i], depth: row.depth + 1 });
      }
    }
  }

  visibleRows = rows;
  treeSpacer.style.height = `${Math.max(1, visibleRows.length * ROW_HEIGHT)}px`;
}

function renderRows(): void {
  if (!visibleRows.length) {
    treeRows.replaceChildren();
    return;
  }

  const scrollTop = treeViewport.scrollTop;
  const viewportHeight = treeViewport.clientHeight;

  const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN_ROWS);
  const end = Math.min(
    visibleRows.length,
    Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN_ROWS
  );

  const fragment = document.createDocumentFragment();

  for (let index = start; index < end; index += 1) {
    const rowInfo = visibleRows[index];
    const node = nodes[rowInfo.id];
    if (!node) continue;

    const row = document.createElement("div");
    row.className = `tree-row${node.id === selectedId ? " selected" : ""}`;
    row.style.top = `${index * ROW_HEIGHT}px`;
    row.dataset.id = String(node.id);

    const nameCell = document.createElement("div");
    nameCell.className = "cell name-cell";
    nameCell.style.paddingLeft = `${8 + rowInfo.depth * 16}px`;

    const twisty = document.createElement("button");
    twisty.type = "button";
    twisty.className = "twisty";
    twisty.disabled = node.childCount === 0;
    twisty.textContent =
      node.childCount === 0 ? "" : expanded.has(node.id) ? "▾" : "▸";

    const icon = document.createElement("span");
    icon.className = "file-icon";
    icon.textContent = node.isDir ? "📁" : "📄";

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

function toggleExpanded(id: number): void {
  if (!nodes[id]) return;

  if (expanded.has(id)) {
    expanded.delete(id);
  } else {
    expanded.add(id);
  }

  rebuildVisibleRows();
  renderRows();
}

function selectNode(id: number, scrollIntoView: boolean, expandSelf: boolean): void {
  if (!nodes[id]) return;

  selectedId = id;
  ensureAncestorsExpanded(id);

  if (expandSelf && nodes[id].isDir) {
    expanded.add(id);
  }

  rebuildVisibleRows();

  if (scrollIntoView) {
    scrollSelectedIntoView();
  }

  renderRows();
  renderTreemap();
  renderSelectedPath();
}

function ensureAncestorsExpanded(id: number): void {
  let current = nodes[id]?.parent;
  while (current != null && nodes[current]) {
    expanded.add(current);
    current = nodes[current].parent;
  }
}

function scrollSelectedIntoView(): void {
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

function renderTreemap(): void {
  treemapRects = drawTreemap(treemapCanvas, nodes, selectedId);
}

function renderSelectedPath(): void {
  if (!nodes[selectedId]) {
    selectedPath.textContent = "未选择";
    return;
  }

  const node = nodes[selectedId];
  selectedPath.textContent = `${nodePath(selectedId)} · ${formatBytes(
    node.totalAllocated
  )}`;
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

  for (let i = 1; i < parts.length; i += 1) {
    if (path.endsWith("\\") || path.endsWith("/")) {
      path += parts[i];
    } else {
      path += `\\${parts[i]}`;
    }
  }

  return path;
}

function renderWarnings(): void {
  warningsEl.replaceChildren();

  if (!result?.warnings.length) return;

  for (const warning of result.warnings) {
    const div = document.createElement("div");
    div.className = "warning-item";
    div.textContent = warning;
    warningsEl.append(div);
  }
}

function setStatus(message: string): void {
  statusText.textContent = message;
}

void bootstrap();
