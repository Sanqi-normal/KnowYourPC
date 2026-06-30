import { listen } from "@tauri-apps/api/event";
import type { PerfSnapshot, ProcessInfo } from "../types";


export function initPerfPanel(container: HTMLElement): () => void {
  container.innerHTML = `<div class="perf-panel"><div class="perf-loading">等待性能数据...</div></div>`;

  let unlisten: (() => void) | null = null;
  let cancelled = false;

  listen<PerfSnapshot>("perf-update", (event) => {
    if (cancelled) return;
    render(container, event.payload);
  }).then((fn) => {
    unlisten = fn;
  });

  return () => {
    cancelled = true;
    if (unlisten) unlisten();
  };
}

function render(container: HTMLElement, data: PerfSnapshot) {
  const panel = container.querySelector(".perf-panel") ?? document.createElement("div");
  panel.className = "perf-panel";

  const gaugeHtml = `
    <div class="perf-grid">
      <div class="perf-gauge-card">
        <div class="perf-gauge-title">CPU</div>
        <div class="perf-gauge-value">${data.cpuPercent.toFixed(1)}%</div>
        <div class="perf-bar"><div class="perf-bar-fill" style="width:${Math.min(data.cpuPercent, 100)}%;background:#58a6ff"></div></div>
      </div>
      <div class="perf-gauge-card">
        <div class="perf-gauge-title">GPU</div>
        <div class="perf-gauge-value">${data.gpuPercent.toFixed(1)}%</div>
        <div class="perf-bar"><div class="perf-bar-fill" style="width:${Math.min(data.gpuPercent, 100)}%;background:#3fb950"></div></div>
      </div>
      <div class="perf-gauge-card">
        <div class="perf-gauge-title">内存</div>
        <div class="perf-gauge-value">${data.memoryPercent.toFixed(1)}%</div>
        <div class="perf-bar"><div class="perf-bar-fill" style="width:${Math.min(data.memoryPercent, 100)}%;background:#d29922"></div></div>
        <div class="perf-bar-label">${data.memoryUsedGb.toFixed(1)} / ${data.memoryTotalGb.toFixed(1)} GB</div>
      </div>
    </div>
    <div class="perf-io-grid">
      <div class="perf-io-card">
        <div class="perf-io-title">磁盘 I/O</div>
        <div class="perf-io-row"><span>读取</span><span class="perf-io-val">${data.diskReadMbps.toFixed(1)} MB/s</span></div>
        <div class="perf-io-row"><span>写入</span><span class="perf-io-val">${data.diskWriteMbps.toFixed(1)} MB/s</span></div>
      </div>
      <div class="perf-io-card">
        <div class="perf-io-title">网络 I/O</div>
        <div class="perf-io-row"><span>下载</span><span class="perf-io-val">${data.netRecvKbps.toFixed(1)} KB/s</span></div>
        <div class="perf-io-row"><span>上传</span><span class="perf-io-val">${data.netSentKbps.toFixed(1)} KB/s</span></div>
      </div>
    </div>
    <div class="perf-proc-section">
      <div class="perf-proc-title">进程 Top 20</div>
      <div class="perf-proc-header">
        <span class="perf-proc-cell">名称</span>
        <span class="perf-proc-cell numeric">CPU</span>
        <span class="perf-proc-cell numeric">内存</span>
        <span class="perf-proc-cell numeric">PID</span>
      </div>
      <div class="perf-proc-list">${
        data.topProcesses.map(p => processRow(p)).join("")
      }</div>
    </div>
  `;

  panel.innerHTML = gaugeHtml;
  if (!panel.parentNode) {
    container.append(panel);
  }
}

function processRow(p: ProcessInfo): string {
  const memStr = p.memoryMb > 1024 ? `${(p.memoryMb / 1024).toFixed(1)} GB` : `${Math.round(p.memoryMb)} MB`;
  return `<div class="perf-proc-row">
    <span class="perf-proc-cell" title="${p.name}">${p.name}</span>
    <span class="perf-proc-cell numeric">${p.cpuPercent.toFixed(1)}%</span>
    <span class="perf-proc-cell numeric">${memStr}</span>
    <span class="perf-proc-cell numeric">${p.pid}</span>
  </div>`;
}
