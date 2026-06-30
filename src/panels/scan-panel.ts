import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  ChildNode, ExtensionStat, ProgressEvent, ScanMode, ScanOptions,
  ScanResult, SearchResult, TreemapItem, VolumeInfo,
} from "../types";
import { formatBytes, formatDuration, formatNumber, formatPercent } from "../format";
import { drawTreemap, hitTestTreemap, hitTestTreemapNode, buildNodePath, type TreemapRect } from "../treemap";

const ROW_HEIGHT = 26;
const OVERSCAN_ROWS = 18;

const TEMPLATE = `
<div class="scan-panel">
  <div id="scanAdminBanner" class="admin-banner hidden">
    <span>\u26a0 \u672a\u4ee5\u7ba1\u7406\u5458\u8eab\u4efd\u8fd0\u884c\uff0cMFT \u6a21\u5f0f\u53ef\u80fd\u4e0d\u53ef\u7528</span>
    <button id="scanRestartAdminBtn" class="admin-restart-btn">\u4ee5\u7ba1\u7406\u5458\u8eab\u4efd\u91cd\u542f</button>
  </div>

  <header class="scan-toolbar">
    <div class="brand"><div class="brand-title">Fast Disk Analyzer</div></div>
    <label class="toolbar-field"><span>\u5377</span><select id="volumeSelect"></select></label>
    <label class="toolbar-field"><span>\u6a21\u5f0f</span>
      <select id="modeSelect">
        <option value="auto">\u81ea\u52a8</option>
        <option value="ntfsMft">NTFS MFT</option>
        <option value="walk">\u517c\u5bb9\u9012\u5f52</option>
      </select>
    </label>
    <button id="refreshVolumesButton" class="secondary-button">\u5237\u65b0\u5377</button>
    <button id="scanButton" class="primary-button">\u5f00\u59cb\u626b\u63cf</button>

    <div id="searchWrap" class="search-wrap hidden">
      <input id="searchInput" class="search-input" type="text" placeholder="\u641c\u7d22\u6587\u4ef6/\u6587\u4ef6\u5939..." />
      <div id="searchResults" class="search-results hidden"></div>
    </div>

    <div class="progress-wrap">
      <div id="progressBar" class="progress-bar"><div id="progressFill" class="progress-fill"></div></div>
      <div id="statusText" class="status-text">\u5c31\u7eea</div>
    </div>
  </header>

  <section id="summaryBar" class="summary-bar">
    <div class="summary-item"><span class="summary-label">\u5360\u7528\u7a7a\u95f4</span><span id="summaryAllocated" class="summary-value">\u2014</span></div>
    <div class="summary-item"><span class="summary-label">\u903b\u8f91\u5927\u5c0f</span><span id="summarySize" class="summary-value">\u2014</span></div>
    <div class="summary-item"><span class="summary-label">\u6587\u4ef6</span><span id="summaryFiles" class="summary-value">\u2014</span></div>
    <div class="summary-item"><span class="summary-label">\u76ee\u5f55</span><span id="summaryDirs" class="summary-value">\u2014</span></div>
    <div class="summary-item"><span class="summary-label">\u626b\u63cf\u5668</span><span id="summaryScanner" class="summary-value">\u2014</span></div>
  </section>

  <main class="scan-main-content">
    <section id="treePanel" class="pane tree-pane">
      <div class="pane-header">
        <span class="pane-title">\u6587\u4ef6\u6811</span>
        <div class="pane-header-right">
          <span id="selectedPath" class="selected-path">\u672a\u9009\u62e9</span>
          <button id="upButton" class="small-button" title="\u4e0a\u4e00\u7ea7">\u2191</button>
        </div>
      </div>
      <div class="table-header">
        <div>\u540d\u79f0</div><div class="numeric">\u5360\u7528</div><div class="numeric">\u5360\u6bd4</div><div class="numeric">\u9879\u76ee</div>
      </div>
      <div id="treeViewport" class="tree-viewport">
        <div id="treeSpacer" class="tree-spacer"><div id="treeRows" class="tree-rows"></div></div>
      </div>
    </section>
    <div id="splitter1" class="splitter splitter-v"></div>
    <section id="vizPanel" class="pane viz-pane">
      <div class="pane-header">
        <span class="pane-title">Treemap</span>
        <div class="pane-header-right"><button id="zoomOutBtn" class="small-button" title="\u8fd4\u56de\u4e0a\u7ea7">\u2190</button></div>
      </div>
      <canvas id="treemapCanvas"></canvas>
      <div id="treemapTooltip" class="treemap-tooltip hidden"></div>
    </section>
    <div id="splitter2" class="splitter splitter-v"></div>
    <section id="extPanel" class="pane ext-pane">
      <div class="pane-header"><span class="pane-title">\u6269\u5c55\u540d</span></div>
      <div id="extList" class="ext-list"></div>
    </section>
  </main>

  <footer id="scanWarnings" class="warnings"></footer>

  <div id="scanContextMenu" class="context-menu hidden">
    <div id="ctxOpen" class="ctx-item">\u5728\u8d44\u6e90\u7ba1\u7406\u5668\u4e2d\u6253\u5f00</div>
    <div id="ctxCopyPath" class="ctx-item">\u590d\u5236\u8def\u5f84</div>
    <div id="ctxCopyName" class="ctx-item">\u590d\u5236\u6587\u4ef6\u540d</div>
  </div>
</div>
`;

export function initScanPanel(container: HTMLElement): () => void {
  container.innerHTML = TEMPLATE;

  const $ = <T extends HTMLElement>(id: string) => container.querySelector<T>(`#${id}`)!;
  const volumeSelect = $<HTMLSelectElement>("volumeSelect");
  const modeSelect = $<HTMLSelectElement>("modeSelect");
  const refreshVolumesButton = $<HTMLButtonElement>("refreshVolumesButton");
  const scanButton = $<HTMLButtonElement>("scanButton");
  const progressFill = $<HTMLDivElement>("progressFill");
  const statusText = $<HTMLDivElement>("statusText");
  const summaryAllocated = $<HTMLDivElement>("summaryAllocated");
  const summarySize = $<HTMLDivElement>("summarySize");
  const summaryFiles = $<HTMLDivElement>("summaryFiles");
  const summaryDirs = $<HTMLDivElement>("summaryDirs");
  const summaryScanner = $<HTMLDivElement>("summaryScanner");
  const selectedPath = $<HTMLDivElement>("selectedPath");
  const upButton = $<HTMLButtonElement>("upButton");
  const zoomOutBtn = $<HTMLButtonElement>("zoomOutBtn");
  const treeViewport = $<HTMLDivElement>("treeViewport");
  const treeSpacer = $<HTMLDivElement>("treeSpacer");
  const treeRows = $<HTMLDivElement>("treeRows");
  const treemapCanvas = $<HTMLCanvasElement>("treemapCanvas");
  const warningsEl = $<HTMLDivElement>("scanWarnings");
  const extList = $<HTMLDivElement>("extList");
  const contextMenu = $<HTMLDivElement>("scanContextMenu");
  const ctxOpen = $<HTMLDivElement>("ctxOpen");
  const ctxCopyPath = $<HTMLDivElement>("ctxCopyPath");
  const ctxCopyName = $<HTMLDivElement>("ctxCopyName");
  const splitter1 = $<HTMLDivElement>("splitter1");
  const splitter2 = $<HTMLDivElement>("splitter2");
  const treePanel = $<HTMLElement>("treePanel");
  const extPanel = $<HTMLElement>("extPanel");
  const adminBanner = $<HTMLDivElement>("scanAdminBanner");
  const restartAdminBtn = $<HTMLButtonElement>("scanRestartAdminBtn");
  const searchWrap = $<HTMLDivElement>("searchWrap");
  const searchInput = $<HTMLInputElement>("searchInput");
  const searchResults = $<HTMLDivElement>("searchResults");
  const treemapTooltip = $<HTMLDivElement>("treemapTooltip");

  let volumes: VolumeInfo[] = [];
  let result: ScanResult | null = null;
  let nodeCache = new Map<number, ChildNode>();
  let parentChildren = new Map<number, number[]>();
  let expanded = new Set<number>([0]);
  let selectedId = 0;
  let visibleRows: { id: number; depth: number }[] = [];
  let treemapRects: TreemapRect[] = [];
  let scanning = false;
  let contextMenuId: number | null = null;
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  function setStatus(msg: string) { statusText.textContent = msg; }

  async function checkAdmin() {
    try {
      if (!(await invoke<boolean>("is_admin"))) {
        adminBanner.classList.remove("hidden");
      }
    } catch { /* ignore */ }
  }

  async function loadVolumes() {
    try {
      setStatus("枚举磁盘卷...");
      volumes = await invoke<VolumeInfo[]>("list_volumes");
      renderVolumeOptions();
      setStatus("就绪");
    } catch (e) { setStatus(`枚举卷失败: ${String(e)}`); }
  }

  function renderVolumeOptions() {
    const prev = volumeSelect.value;
    volumeSelect.replaceChildren();
    for (const v of volumes) {
      const opt = document.createElement("option");
      opt.value = v.root;
      opt.textContent = `${v.displayName}${v.fsName ? " " + v.fsName : ""}${v.totalBytes > 0 ? " " + formatBytes(v.totalBytes) : ""}`;
      volumeSelect.append(opt);
    }
    if (prev && volumes.some(v => v.root === prev)) { volumeSelect.value = prev; return; }
    const preferred = volumes.find(v => v.ntfsCandidate && v.driveType === "Fixed") ?? volumes.find(v => v.ntfsCandidate) ?? volumes[0];
    if (preferred) volumeSelect.value = preferred.root;
  }

  async function loadChildren(parentId: number): Promise<ChildNode[]> {
    const children = await invoke<ChildNode[]>("get_children", { parentId });
    const ids: number[] = [];
    for (const c of children) { nodeCache.set(c.id, c); ids.push(c.id); }
    parentChildren.set(parentId, ids);
    return children;
  }

  async function ensureNodeInCache(id: number) {
    if (nodeCache.has(id)) return;
    const ancestors = await invoke<ChildNode[]>("get_node_with_ancestors", { nodeId: id });
    for (const n of ancestors) { if (!nodeCache.has(n.id)) nodeCache.set(n.id, n); }
  }

  async function ensureTreemapNodesInCache(rects: TreemapRect[]) {
    const missing = new Set<number>();
    for (const r of rects) { if (!nodeCache.has(r.id)) missing.add(r.id); }
    if (missing.size === 0) return;
    const results = await Promise.all([...missing].map(id =>
      invoke<ChildNode[]>("get_node_with_ancestors", { nodeId: id }).catch(() => [] as ChildNode[])
    ));
    for (const nodes of results) {
      for (const n of nodes) { if (!nodeCache.has(n.id)) nodeCache.set(n.id, n); }
    }
  }

  function renderProgress(p: ProgressEvent) {
    if (p.total && p.total > 0) progressFill.style.width = `${Math.min(100, (p.processed / p.total) * 100)}%`;
    setStatus(p.message);
  }

  function renderSummary() {
    if (!result) {
      summaryAllocated.textContent = summarySize.textContent = summaryFiles.textContent = summaryDirs.textContent = summaryScanner.textContent = "\u2014";
      return;
    }
    summaryAllocated.textContent = formatBytes(result.totalAllocated);
    summarySize.textContent = formatBytes(result.totalSize);
    summaryFiles.textContent = formatNumber(result.fileCount);
    summaryDirs.textContent = formatNumber(result.dirCount);
    summaryScanner.textContent = `${result.scanner} ${formatDuration(result.elapsedMs)}`;
  }

  function renderWarnings() {
    warningsEl.replaceChildren();
    if (!result?.warnings.length) return;
    for (const w of result.warnings) {
      const d = document.createElement("div"); d.className = "warning-item"; d.textContent = w;
      warningsEl.append(d);
    }
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
        if (childIds) { for (let i = childIds.length - 1; i >= 0; i--) stack.push({ id: childIds[i], depth: row.depth + 1 }); }
      }
    }
    visibleRows = rows;
    treeSpacer.style.height = `${Math.max(1, visibleRows.length * ROW_HEIGHT)}px`;
  }

  function renderRows() {
    if (!visibleRows.length) { treeRows.replaceChildren(); return; }
    const scrollTop = treeViewport.scrollTop;
    const vh = treeViewport.clientHeight;
    const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN_ROWS);
    const end = Math.min(visibleRows.length, Math.ceil((scrollTop + vh) / ROW_HEIGHT) + OVERSCAN_ROWS);
    const frag = document.createDocumentFragment();

    for (let i = start; i < end; i++) {
      const ri = visibleRows[i];
      const node = nodeCache.get(ri.id);
      if (!node) continue;

      const row = document.createElement("div");
      row.className = `tree-row${node.id === selectedId ? " selected" : ""}`;
      row.style.top = `${i * ROW_HEIGHT}px`;
      row.dataset.id = String(node.id);

      const nameCell = document.createElement("div");
      nameCell.className = "cell name-cell";
      nameCell.style.paddingLeft = `${8 + ri.depth * 14}px`;

      const twisty = document.createElement("button");
      twisty.type = "button"; twisty.className = "twisty";
      twisty.disabled = node.childCount === 0;
      twisty.textContent = node.childCount === 0 ? "" : expanded.has(node.id) ? "\u25be" : "\u25b8";

      const icon = document.createElement("span");
      icon.className = "file-icon";
      icon.textContent = node.isDir ? "\uD83D\uDCC1" : "\uD83D\uDCC4";

      const nameText = document.createElement("span");
      nameText.textContent = node.name;
      nameCell.append(twisty, icon, nameText);

      const pn = node.parent != null ? nodeCache.get(node.parent) : null;
      const pt = pn ? pn.totalAllocated : result?.totalAllocated;
      const pct = pt ? (node.totalAllocated / pt) * 100 : 0;

      const pctCell = document.createElement("div");
      pctCell.className = "cell pct-cell";
      pctCell.innerHTML = `<div class="pct-bar"><div class="pct-bar-fill" style="width:${pct}%"></div><span class="pct-bar-label">${formatPercent(pct)}</span></div>`;

      function numCell(text: string) { const c = document.createElement("div"); c.className = "cell numeric"; c.textContent = text; return c; }

      row.append(nameCell, numCell(formatBytes(node.totalAllocated)), pctCell, numCell(formatNumber(node.fileCount + node.dirCount)));
      frag.append(row);
    }
    treeRows.replaceChildren(frag);
  }

  async function toggleExpanded(id: number) {
    const node = nodeCache.get(id);
    if (!node) return;
    if (expanded.has(id)) { expanded.delete(id); }
    else { if (node.childCount > 0) await loadChildren(id); expanded.add(id); }
    rebuildVisibleRows(); renderRows();
  }

  async function selectNode(id: number, scroll: boolean, expandSelf: boolean) {
    const node = nodeCache.get(id);
    if (!node) return;
    selectedId = id;
    const path: number[] = [];
    let cur = nodeCache.get(id)?.parent;
    while (cur != null && nodeCache.has(cur)) { path.push(cur); cur = nodeCache.get(cur)?.parent; }
    for (let i = path.length - 1; i >= 0; i--) {
      const pid = path[i];
      if (!expanded.has(pid)) { await loadChildren(pid); expanded.add(pid); }
    }
    if (expandSelf && node.isDir) { if (node.childCount > 0) await loadChildren(id); expanded.add(id); }
    rebuildVisibleRows();
    if (scroll) {
      const idx = visibleRows.findIndex(r => r.id === selectedId);
      if (idx >= 0) {
        const rt = idx * ROW_HEIGHT; const rb = rt + ROW_HEIGHT;
        if (rt < treeViewport.scrollTop) treeViewport.scrollTop = rt;
        else if (rb > treeViewport.scrollTop + treeViewport.clientHeight) treeViewport.scrollTop = rb - treeViewport.clientHeight;
      }
    }
    renderRows();
    await renderTreemap();
    const pathStr = await nodePath(selectedId);
    selectedPath.textContent = `${pathStr} ${formatBytes(node.totalAllocated)}`;
  }

  async function renderTreemap() {
    if (!nodeCache.has(selectedId)) return;
    try {
      const items = await invoke<TreemapItem[]>("get_treemap_data", { rootId: selectedId, maxItems: 3000 });
      treemapRects = drawTreemap(treemapCanvas, items);
      ensureTreemapNodesInCache(treemapRects);
    } catch { treemapRects = []; }
  }

  async function nodePath(id: number): Promise<string> {
    if (!nodeCache.has(id)) return "";
    return await invoke<string>("get_node_path", { nodeId: id });
  }

  async function loadExtensionStats() {
    try {
      const stats = await invoke<ExtensionStat[]>("get_extension_stats");
      extList.replaceChildren();
      if (stats.length === 0) return;
      const total = stats.reduce((s, st) => s + st.allocated, 0);
      if (total <= 0) return;
      for (const stat of stats) {
        const item = document.createElement("div"); item.className = "ext-item";
        const pct = (stat.allocated / total) * 100;
        const ext = stat.extension || "(无)";
        let hash = 2166136261; for (let j = 0; j < ext.length; j++) { hash ^= ext.charCodeAt(j); hash = Math.imul(hash, 16777619); }
        const hue = Math.abs(hash) % 360;
        item.innerHTML = `<span class="ext-name">.${ext}</span><div class="ext-bar-wrap"><div class="ext-bar-fill" style="width:${pct}%;background:hsl(${hue},55%,50%)"></div><span class="ext-bar-label">${formatBytes(stat.allocated)} (${formatNumber(stat.fileCount)})</span></div><span class="ext-size">${pct.toFixed(1)}%</span>`;
        extList.append(item);
      }
    } catch { /* ignore */ }
  }

  async function startScan() {
    if (scanning) return;
    const root = volumeSelect.value;
    if (!root) { setStatus("没有可扫描的卷"); return; }
    scanning = true; scanButton.disabled = true; refreshVolumesButton.disabled = true;
    progressFill.style.width = "0%"; setStatus("准备扫描..."); extList.replaceChildren();
    try {
      result = await invoke<ScanResult>("scan", { options: { root, mode: modeSelect.value as ScanMode, includeSystemFiles: true } as ScanOptions });
      nodeCache.clear(); parentChildren.clear(); selectedId = 0; expanded = new Set<number>([0]);
      const rootChildren = await loadChildren(0);
      nodeCache.set(0, { id: 0, parent: null, name: result.root, isDir: true, size: 0, allocated: 0, totalSize: result.totalSize, totalAllocated: result.totalAllocated, childCount: rootChildren.length, fileCount: result.fileCount, dirCount: result.dirCount, extension: null });
      rebuildVisibleRows(); renderSummary(); renderWarnings();
      await selectNode(0, false, true); loadExtensionStats();
      searchWrap.classList.remove("hidden"); searchInput.value = ""; searchResults.classList.add("hidden");
      progressFill.style.width = "100%"; setStatus(`扫描完成 (${formatDuration(result.elapsedMs)})`);
    } catch (e) { setStatus(`扫描失败: ${String(e)}`); }
    finally { scanning = false; scanButton.disabled = false; refreshVolumesButton.disabled = false; }
  }

  function showContextMenu(x: number, y: number, id: number) {
    contextMenuId = id; contextMenu.style.left = `${x}px`; contextMenu.style.top = `${y}px`; contextMenu.classList.remove("hidden");
  }

  function hideContextMenu() { contextMenu.classList.add("hidden"); contextMenuId = null; }

  async function doSearch(query: string) {
    try {
      const items = await invoke<SearchResult[]>("search_files", { query, maxResults: 100 });
      searchResults.replaceChildren();
      if (items.length === 0) {
        const d = document.createElement("div"); d.className = "search-result-item"; d.textContent = "未找到匹配项";
        searchResults.append(d); searchResults.classList.remove("hidden"); return;
      }
      for (const item of items.slice(0, 100)) {
        const row = document.createElement("div"); row.className = "search-result-item";
        const path = item.path.length <= 80 ? item.path : "..." + item.path.slice(-77);
        row.innerHTML = `<span class="search-result-icon">${item.isDir ? "\uD83D\uDCC1" : "\uD83D\uDCC4"}</span><span class="search-result-name">${item.name.replace(/[&<>"']/g, c => ({ "&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;" }[c] ?? c))}</span><span class="search-result-size">${formatBytes(item.totalAllocated)}</span><span class="search-result-path">${path}</span>`;
        row.addEventListener("click", async () => { searchResults.classList.add("hidden"); searchInput.blur(); await selectNode(item.id, true, item.isDir); });
        row.addEventListener("mousedown", e => e.preventDefault());
        searchResults.append(row);
      }
      searchResults.classList.remove("hidden");
    } catch { searchResults.classList.add("hidden"); }
  }

  async function openInExplorer(id: number) {
    const node = nodeCache.get(id); if (!node) return;
    const path = await nodePath(id);
    try { await invoke("open_in_explorer", { path }); } catch (e) { setStatus(`打开失败: ${e}`); }
  }

  async function copyNodePath(id: number) { try { await navigator.clipboard.writeText(await nodePath(id)); } catch { /* */ } }
  async function copyNodeName(id: number) { const n = nodeCache.get(id); if (n) try { await navigator.clipboard.writeText(n.name); } catch { /* */ } }

  function initSplitters() {
    function makeSplitter(s: HTMLElement, left: HTMLElement | null, right: HTMLElement | null) {
      let dragging = false, startX = 0, startLeft = 0, startRight = 0;
      s.addEventListener("mousedown", e => { dragging = true; startX = e.clientX; startLeft = left ? left.getBoundingClientRect().width : 0; startRight = right ? right.getBoundingClientRect().width : 0; s.classList.add("active"); document.body.style.cursor = "col-resize"; document.body.style.userSelect = "none"; });
      document.addEventListener("mousemove", e => { if (!dragging) return; const dx = e.clientX - startX; if (left) left.style.flex = `0 0 ${Math.max(200, Math.min(800, startLeft + dx))}px`; if (right) right.style.flex = `0 0 ${Math.max(160, Math.min(500, startRight - dx))}px`; });
      document.addEventListener("mouseup", () => { if (dragging) { dragging = false; s.classList.remove("active"); document.body.style.cursor = ""; document.body.style.userSelect = ""; } });
    }
    makeSplitter(splitter1, treePanel, null);
    makeSplitter(splitter2, null, extPanel);
  }

  // Wire events
  refreshVolumesButton.addEventListener("click", loadVolumes);
  scanButton.addEventListener("click", startScan);
  restartAdminBtn.addEventListener("click", async () => {
    restartAdminBtn.disabled = true; restartAdminBtn.textContent = "正在重启...";
    try { await invoke("restart_as_admin"); } catch { restartAdminBtn.disabled = false; restartAdminBtn.textContent = "以管理员身份重启"; }
  });
  upButton.addEventListener("click", async () => { const c = nodeCache.get(selectedId); if (c?.parent != null) await selectNode(c.parent, true, false); });
  zoomOutBtn.addEventListener("click", async () => { const c = nodeCache.get(selectedId); if (c?.parent != null) await selectNode(c.parent, true, true); });
  treeViewport.addEventListener("scroll", renderRows);

  treeRows.addEventListener("click", async e => {
    const row = (e.target as HTMLElement).closest<HTMLElement>(".tree-row"); if (!row) return;
    const id = Number(row.dataset.id); if (!Number.isInteger(id) || !nodeCache.has(id)) return;
    if ((e.target as HTMLElement).closest(".twisty")) { await toggleExpanded(id); return; }
    await selectNode(id, false, false);
  });

  treeRows.addEventListener("mousemove", e => {
    const row = (e.target as HTMLElement).closest<HTMLElement>(".tree-row");
    if (!row) { treemapTooltip.classList.add("hidden"); return; }
    const id = Number(row.dataset.id); if (!Number.isInteger(id) || !nodeCache.has(id)) { treemapTooltip.classList.add("hidden"); return; }
    const node = nodeCache.get(id)!; const path = buildNodePath(id, nodeCache, 0);
    treemapTooltip.textContent = `${path}  ${formatBytes(node.totalAllocated)}`;
    treemapTooltip.style.left = `${e.clientX + 12}px`; treemapTooltip.style.top = `${e.clientY + 12}px`;
    treemapTooltip.classList.remove("hidden");
  });
  treeRows.addEventListener("mouseleave", () => treemapTooltip.classList.add("hidden"));

  treeRows.addEventListener("contextmenu", e => {
    const row = (e.target as HTMLElement).closest<HTMLElement>(".tree-row"); if (!row) return;
    const id = Number(row.dataset.id); if (!Number.isInteger(id) || !nodeCache.has(id)) return;
    e.preventDefault(); showContextMenu(e.clientX, e.clientY, id);
  });

  treemapCanvas.addEventListener("click", async e => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = e.clientX - rect.left, y = e.clientY - rect.top;
    const hit = hitTestTreemap(treemapRects, x, y);
    if (hit != null) { if (!nodeCache.has(hit)) await ensureNodeInCache(hit); if (nodeCache.has(hit)) { const hn = nodeCache.get(hit)!; await selectNode(hit, true, hn.isDir); } }
  });

  treemapCanvas.addEventListener("contextmenu", async e => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = e.clientX - rect.left, y = e.clientY - rect.top;
    const hit = hitTestTreemapNode(treemapRects, x, y);
    if (hit) { if (!nodeCache.has(hit.id)) await ensureNodeInCache(hit.id); if (nodeCache.has(hit.id)) { e.preventDefault(); showContextMenu(e.clientX, e.clientY, hit.id); } }
  });

  treemapCanvas.addEventListener("mousemove", async e => {
    const rect = treemapCanvas.getBoundingClientRect();
    const x = e.clientX - rect.left, y = e.clientY - rect.top;
    const hit = hitTestTreemapNode(treemapRects, x, y);
    if (hit) { if (!nodeCache.has(hit.id)) await ensureNodeInCache(hit.id); if (nodeCache.has(hit.id)) { const path = buildNodePath(hit.id, nodeCache, selectedId); treemapTooltip.textContent = `${path}  ${formatBytes(hit.item.size)}`; treemapTooltip.style.left = `${e.clientX + 12}px`; treemapTooltip.style.top = `${e.clientY + 12}px`; treemapTooltip.classList.remove("hidden"); } else { treemapTooltip.classList.add("hidden"); } } else { treemapTooltip.classList.add("hidden"); }
  });
  treemapCanvas.addEventListener("mouseleave", () => treemapTooltip.classList.add("hidden"));

  document.addEventListener("click", hideContextMenu);
  ctxOpen.addEventListener("click", () => { if (contextMenuId != null) openInExplorer(contextMenuId); hideContextMenu(); });
  ctxCopyPath.addEventListener("click", () => { if (contextMenuId != null) copyNodePath(contextMenuId); hideContextMenu(); });
  ctxCopyName.addEventListener("click", () => { if (contextMenuId != null) copyNodeName(contextMenuId); hideContextMenu(); });

  searchInput.addEventListener("input", () => {
    if (searchTimeout) clearTimeout(searchTimeout);
    const q = searchInput.value.trim();
    if (q.length < 2) { searchResults.classList.add("hidden"); return; }
    searchTimeout = setTimeout(() => doSearch(q), 200);
  });
  searchInput.addEventListener("blur", () => setTimeout(() => searchResults.classList.add("hidden"), 200));
  searchInput.addEventListener("focus", () => { if (searchInput.value.trim().length >= 2) searchResults.classList.remove("hidden"); });

  window.addEventListener("resize", () => { renderRows(); renderTreemap(); });
  new ResizeObserver(() => renderTreemap()).observe(treemapCanvas);

  initSplitters();

  // Bootstrap
  checkAdmin();
  let unlistenProgress: (() => void) | null = null;
  listen<ProgressEvent>("scan-progress", e => { renderProgress(e.payload); }).then(fn => { unlistenProgress = fn; });
  loadVolumes();

  return () => {
    if (unlistenProgress) unlistenProgress();
    container.innerHTML = "";
  };
}
