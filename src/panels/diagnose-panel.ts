import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { HardwareInfo, PerfSnapshot, ProcessInfo } from "../types";

export function initDiagnosePanel(container: HTMLElement): () => void {
  container.innerHTML = `<div class="diag-panel"><div class="diag-loading">加载中...</div></div>`;

  let unlisten: (() => void) | null = null;
  let cancelled = false;

  // Hardware info (static, cached by backend)
  invoke<HardwareInfo>("get_hardware_info")
    .then((info) => {
      if (cancelled) return;
      renderStatic(container, info);
    })
    .catch((err) => {
      if (cancelled) return;
      container.innerHTML = `<div class="diag-panel"><div class="diag-error">加载硬件信息失败: ${err}</div></div>`;
    });

  // Performance data (live updates)
  listen<PerfSnapshot>("perf-update", (event) => {
    if (cancelled) return;
    renderPerf(container, event.payload);
  }).then((fn) => {
    unlisten = fn;
  });

  return () => {
    cancelled = true;
    if (unlisten) unlisten();
  };
}

function renderStatic(container: HTMLElement, info: HardwareInfo) {
  const panel = document.createElement("div");
  panel.className = "diag-panel";

  const cpuGaugeId = "dgCpuGauge";
  const cpuValId = "dgCpuVal";
  const memGaugeId = "dgMemGauge";
  const memValId = "dgMemVal";
  const memLabelId = "dgMemLabel";
  const gpuGaugeId = "dgGpuGauge";
  const gpuValId = "dgGpuVal";
  const netRecvId = "dgNetRecv";
  const netSentId = "dgNetSent";
  const procListId = "dgProcList";

  // ── CPU ──
  const cpuRows: [string, string][] = [
    ["型号", info.cpu.name],
    ["架构", info.cpu.architecture],
    ["物理核心", `${info.cpu.physicalCores}`],
    ["逻辑线程", `${info.cpu.logicalThreads}`],
    ["主频", info.cpu.frequencyMhz > 0 ? `${info.cpu.frequencyMhz} MHz` : "未知"],
    ["L1 缓存", info.cpu.l1CacheKb != null ? `${info.cpu.l1CacheKb} KB` : "未知"],
    ["L2 缓存", info.cpu.l2CacheKb != null ? `${info.cpu.l2CacheKb} KB` : "未知"],
    ["L3 缓存", info.cpu.l3CacheKb != null ? `${info.cpu.l3CacheKb} KB` : "未知"],
  ];
  panel.append(section("CPU 处理器", cpuRows, gaugeHtml(cpuValId, cpuGaugeId, "0.0")));

  // ── Memory ──
  const memRows: [string, string][] = [["总容量", `${info.ram.totalGb.toFixed(2)} GB`]];
  for (const slot of info.ram.slots) {
    memRows.push([slot.slot, `${slot.capacityGb.toFixed(1)} GB ${slot.memoryType} @ ${slot.speedMhz} MHz`]);
    if (slot.manufacturer !== "未知") {
      memRows.push([`${slot.slot} 厂商`, slot.manufacturer]);
    }
  }
  panel.append(section("内存 (RAM)", memRows, gaugeHtml(memValId, memGaugeId, "0.0", memLabelId)));

  // ── GPU ──
  for (const gpu of info.gpus) {
    const gpuRows: [string, string][] = [
      ["型号", gpu.name],
      ["显存", `${(gpu.vramMb / 1024).toFixed(1)} GB`],
      ["驱动版本", gpu.driverVersion],
    ];
    panel.append(section("显卡 (GPU)", gpuRows, gaugeHtml(gpuValId, gpuGaugeId, "0.0")));
  }

  // ── Network I/O ──
  panel.append(netSection(netRecvId, netSentId));

  // ── Motherboard & BIOS ──
  const mbRows: [string, string][] = [
    ["主板厂商", info.motherboard.manufacturer],
    ["主板型号", info.motherboard.product],
    ["BIOS 厂商", info.bios.manufacturer],
    ["BIOS 版本", info.bios.version],
    ["BIOS 日期", info.bios.releaseDate],
  ];
  panel.append(section("主板 & BIOS", mbRows));

  // ── Battery ──
  if (info.battery.present) {
    const designStr = info.battery.designCapacityMwh != null ? `${(info.battery.designCapacityMwh / 1000).toFixed(0)} mWh` : "未知";
    const fullStr = info.battery.fullChargeCapacityMwh != null ? `${(info.battery.fullChargeCapacityMwh / 1000).toFixed(0)} mWh` : "未知";
    const cycleStr = info.battery.cycleCount != null ? `${info.battery.cycleCount}` : "未知";
    const healthStr = info.battery.healthPercent > 0 ? `${info.battery.healthPercent.toFixed(1)}%` : "未知";
    const barWidth = info.battery.healthPercent > 0 ? Math.min(info.battery.healthPercent, 100) : 0;
    const barColor = info.battery.healthPercent > 80 ? "#3fb950" : info.battery.healthPercent > 50 ? "#d29922" : "#f85149";
    panel.append(batteryCard(designStr, fullStr, cycleStr, healthStr, barWidth, barColor));
  }

  // ── Processes ──
  panel.append(procSection(procListId));

  container.replaceChildren(panel);
}

function section(title: string, rows: [string, string][], gaugeHtml?: string): HTMLElement {
  const el = document.createElement("div");
  el.className = "diag-card";

  let bodyHtml = rows.map(([l, v]) =>
    `<div class="diag-row"><span class="diag-label">${l}</span><span class="diag-val">${v}</span></div>`
  ).join("");

  if (gaugeHtml) {
    bodyHtml += gaugeHtml;
  }

  el.innerHTML = `
    <div class="diag-card-header"><span class="diag-card-title">${title}</span></div>
    <div class="diag-card-body">${bodyHtml}</div>
  `;
  return el;
}

function gaugeHtml(valId: string, barId: string, initial: string, labelId?: string): string {
  return `
    <div class="diag-gauge">
      <div class="diag-gauge-value" id="${valId}">${initial}%</div>
      <div class="diag-bar"><div class="diag-bar-fill" id="${barId}" style="width:0%"></div></div>
      ${labelId ? `<div class="diag-bar-label" id="${labelId}">0.0 / 0.0 GB</div>` : ""}
    </div>
  `;
}

function netSection(recvId: string, sentId: string): HTMLElement {
  const el = document.createElement("div");
  el.className = "diag-card";
  el.innerHTML = `
    <div class="diag-card-header"><span class="diag-card-title">网络 I/O</span></div>
    <div class="diag-card-body">
      <div class="diag-row"><span class="diag-label">下载</span><span class="diag-val" id="${recvId}">0.0 KB/s</span></div>
      <div class="diag-row"><span class="diag-label">上传</span><span class="diag-val" id="${sentId}">0.0 KB/s</span></div>
    </div>
  `;
  return el;
}

function procSection(listId: string): HTMLElement {
  const el = document.createElement("div");
  el.className = "diag-card";
  el.innerHTML = `
    <div class="diag-card-header"><span class="diag-card-title">进程 Top 20</span></div>
    <div class="diag-card-body" style="padding:0">
      <div class="diag-proc-header">
        <span>名称</span><span class="numeric">CPU</span><span class="numeric">内存</span><span class="numeric">PID</span>
      </div>
      <div id="${listId}" class="diag-proc-list"></div>
    </div>
  `;
  return el;
}

function batteryCard(design: string, full: string, cycle: string, health: string, barWidth: number, barColor: string): HTMLElement {
  const el = document.createElement("div");
  el.className = "diag-card";
  el.innerHTML = `
    <div class="diag-card-header"><span class="diag-card-title">电池</span></div>
    <div class="diag-card-body">
      <div class="diag-row"><span class="diag-label">设计容量</span><span class="diag-val">${design}</span></div>
      <div class="diag-row"><span class="diag-label">当前满充</span><span class="diag-val">${full}</span></div>
      <div class="diag-row"><span class="diag-label">循环计数</span><span class="diag-val">${cycle}</span></div>
      <div class="diag-row"><span class="diag-label">电池健康</span><span class="diag-val">${health}</span></div>
      <div class="diag-bat-bar"><div class="diag-bat-fill" style="width:${barWidth}%;background:${barColor}"></div></div>
    </div>
  `;
  return el;
}

// ── Live performance updates ──

function renderPerf(container: HTMLElement, data: PerfSnapshot) {
  // CPU gauge
  const cpuVal = container.querySelector("#dgCpuVal");
  const cpuGauge = container.querySelector("#dgCpuGauge") as HTMLElement | null;
  if (cpuVal) cpuVal.textContent = `${data.cpuPercent.toFixed(1)}%`;
  if (cpuGauge) cpuGauge.style.width = `${Math.min(data.cpuPercent, 100)}%`;

  // Memory gauge
  const memVal = container.querySelector("#dgMemVal");
  const memGauge = container.querySelector("#dgMemGauge") as HTMLElement | null;
  const memLabel = container.querySelector("#dgMemLabel");
  if (memVal) memVal.textContent = `${data.memoryPercent.toFixed(1)}%`;
  if (memGauge) memGauge.style.width = `${Math.min(data.memoryPercent, 100)}%`;
  if (memLabel) memLabel.textContent = `${data.memoryUsedGb.toFixed(1)} / ${data.memoryTotalGb.toFixed(1)} GB`;

  // GPU gauge (placeholder)
  const gpuVal = container.querySelector("#dgGpuVal");
  const gpuGauge = container.querySelector("#dgGpuGauge") as HTMLElement | null;
  if (gpuVal) gpuVal.textContent = `${data.gpuPercent.toFixed(1)}%`;
  if (gpuGauge) gpuGauge.style.width = `${Math.min(data.gpuPercent, 100)}%`;

  // Network I/O
  const netRecv = container.querySelector("#dgNetRecv");
  const netSent = container.querySelector("#dgNetSent");
  if (netRecv) netRecv.textContent = `${data.netRecvKbps.toFixed(1)} KB/s`;
  if (netSent) netSent.textContent = `${data.netSentKbps.toFixed(1)} KB/s`;

  // Processes
  const procList = container.querySelector("#dgProcList");
  if (procList) {
    procList.innerHTML = data.topProcesses.map(p => processRow(p)).join("");
  }
}

function processRow(p: ProcessInfo): string {
  const memStr = p.memoryMb > 1024 ? `${(p.memoryMb / 1024).toFixed(1)} GB` : `${Math.round(p.memoryMb)} MB`;
  return `<div class="diag-proc-row">
    <span title="${p.name}">${p.name}</span>
    <span class="numeric">${p.cpuPercent.toFixed(1)}%</span>
    <span class="numeric">${memStr}</span>
    <span class="numeric">${p.pid}</span>
  </div>`;
}
