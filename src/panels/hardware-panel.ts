import { invoke } from "@tauri-apps/api/core";
import type { HardwareInfo } from "../types";


export function initHardwarePanel(container: HTMLElement): () => void {
  container.innerHTML = `<div class="hw-panel"><div class="hw-loading">加载硬件信息...</div></div>`;

  let cancelled = false;

  invoke<HardwareInfo>("get_hardware_info")
    .then((info) => {
      if (cancelled) return;
      render(container, info);
    })
    .catch((err) => {
      if (cancelled) return;
      container.innerHTML = `<div class="hw-panel"><div class="hw-error">加载失败: ${err}</div></div>`;
    });

  return () => {
    cancelled = true;
  };
}

function render(container: HTMLElement, info: HardwareInfo) {
  const panel = document.createElement("div");
  panel.className = "hw-panel";

  panel.append(cpuCard(info.cpu));
  panel.append(ramCard(info.ram));
  for (const gpu of info.gpus) {
    panel.append(gpuCard(gpu));
  }
  panel.append(motherboardCard(info.motherboard, info.bios));
  if (info.battery.present) {
    panel.append(batteryCard(info.battery));
  }

  container.replaceChildren(panel);
}

function card(title: string, rows: [string, string][]): HTMLElement {
  const el = document.createElement("div");
  el.className = "hw-card";
  el.innerHTML = `<div class="hw-card-header"><span class="hw-card-title">${title}</span></div><div class="hw-card-body"></div>`;
  const body = el.querySelector(".hw-card-body")!;
  for (const [label, value] of rows) {
    const row = document.createElement("div");
    row.className = "hw-row";
    row.innerHTML = `<span class="hw-label">${label}</span><span class="hw-value">${value}</span>`;
    body.append(row);
  }
  return el;
}

function cpuCard(cpu: HardwareInfo["cpu"]): HTMLElement {
  return card("CPU 处理器", [
    ["型号", cpu.name],
    ["架构", cpu.architecture],
    ["物理核心", `${cpu.physicalCores}`],
    ["逻辑线程", `${cpu.logicalThreads}`],
    ["主频", cpu.frequencyMhz > 0 ? `${cpu.frequencyMhz} MHz` : "未知"],
    ["L1 缓存", cpu.l1CacheKb != null ? `${cpu.l1CacheKb} KB` : "未知"],
    ["L2 缓存", cpu.l2CacheKb != null ? `${cpu.l2CacheKb} KB` : "未知"],
    ["L3 缓存", cpu.l3CacheKb != null ? `${cpu.l3CacheKb} KB` : "未知"],
  ]);
}

function ramCard(ram: HardwareInfo["ram"]): HTMLElement {
  const rows: [string, string][] = [["总容量", `${ram.totalGb.toFixed(2)} GB`]];
  if (ram.slots.length === 0) {
    rows.push(["插槽", "无信息"]);
  }
  for (const slot of ram.slots) {
    rows.push([`${slot.slot}`, `${slot.capacityGb.toFixed(1)} GB ${slot.memoryType} @ ${slot.speedMhz} MHz`]);
    if (slot.manufacturer !== "未知") {
      rows.push([`${slot.slot} 厂商`, slot.manufacturer]);
    }
  }
  return card("内存 (RAM)", rows);
}

function gpuCard(gpu: HardwareInfo["gpus"][number]): HTMLElement {
  return card("显卡 (GPU)", [
    ["型号", gpu.name],
    ["显存", `${(gpu.vramMb / 1024).toFixed(1)} GB`],
    ["驱动版本", gpu.driverVersion],
  ]);
}

function motherboardCard(mb: HardwareInfo["motherboard"], bios: HardwareInfo["bios"]): HTMLElement {
  return card("主板 & BIOS", [
    ["主板厂商", mb.manufacturer],
    ["主板型号", mb.product],
    ["BIOS 厂商", bios.manufacturer],
    ["BIOS 版本", bios.version],
    ["BIOS 日期", bios.releaseDate],
  ]);
}

function batteryCard(bat: HardwareInfo["battery"]): HTMLElement {
  const designStr = bat.designCapacityMwh != null ? `${(bat.designCapacityMwh / 1000).toFixed(0)} mWh` : "未知";
  const fullStr = bat.fullChargeCapacityMwh != null ? `${(bat.fullChargeCapacityMwh / 1000).toFixed(0)} mWh` : "未知";
  const cycleStr = bat.cycleCount != null ? `${bat.cycleCount}` : "未知";
  const healthStr = bat.healthPercent > 0 ? `${bat.healthPercent.toFixed(1)}%` : "未知";
  const barWidth = bat.healthPercent > 0 ? Math.min(bat.healthPercent, 100) : 0;
  const barColor = bat.healthPercent > 80 ? "#3fb950" : bat.healthPercent > 50 ? "#d29922" : "#f85149";

  const el = document.createElement("div");
  el.className = "hw-card";
  el.innerHTML = `
    <div class="hw-card-header"><span class="hw-card-title">电池</span></div>
    <div class="hw-card-body">
      <div class="hw-row"><span class="hw-label">设计容量</span><span class="hw-value">${designStr}</span></div>
      <div class="hw-row"><span class="hw-label">当前满充</span><span class="hw-value">${fullStr}</span></div>
      <div class="hw-row"><span class="hw-label">循环计数</span><span class="hw-value">${cycleStr}</span></div>
      <div class="hw-row"><span class="hw-label">电池健康</span><span class="hw-value">${healthStr}</span></div>
      <div class="hw-battery-bar"><div class="hw-battery-fill" style="width:${barWidth}%;background:${barColor}"></div></div>
    </div>`;
  return el;
}
