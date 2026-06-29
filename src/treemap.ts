import type { NodeDto } from "./types";
import { formatBytes } from "./format";

export interface TreemapRect {
  id: number;
  x: number;
  y: number;
  w: number;
  h: number;
  depth: number;
  node: NodeDto;
}

interface Bounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface LayoutItem {
  id: number;
  size: number;
  node: NodeDto;
}

const MAX_VISIBLE = 3000;

export function drawTreemap(
  canvas: HTMLCanvasElement,
  nodes: NodeDto[],
  rootId: number
): TreemapRect[] {
  const width = Math.max(1, canvas.clientWidth);
  const height = Math.max(1, canvas.clientHeight);
  const dpr = window.devicePixelRatio || 1;

  const bitmapWidth = Math.floor(width * dpr);
  const bitmapHeight = Math.floor(height * dpr);

  if (canvas.width !== bitmapWidth || canvas.height !== bitmapHeight) {
    canvas.width = bitmapWidth;
    canvas.height = bitmapHeight;
  }

  const ctx = canvas.getContext("2d");
  if (!ctx) return [];

  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, width, height);

  const root = nodes[rootId];
  if (!root) {
    drawEmpty(ctx, width, height, "暂无数据");
    return [];
  }

  if (!root.children.length || root.totalAllocated <= 0) {
    drawEmpty(ctx, width, height, "此节点没有可视化子项");
    return [];
  }

  const items = collectItems(nodes, rootId);
  if (items.length === 0) {
    drawEmpty(ctx, width, height, "无可视化数据");
    return [];
  }

  items.sort((a, b) => b.size - a.size);

  let layoutItems: LayoutItem[];
  if (items.length > MAX_VISIBLE) {
    layoutItems = items.slice(0, MAX_VISIBLE);
  } else {
    layoutItems = items;
  }

  if (layoutItems.length === 0) {
    drawEmpty(ctx, width, height, "无可视化数据");
    return [];
  }

  const total = layoutItems.reduce((s, i) => s + i.size, 0);
  if (total <= 0) {
    drawEmpty(ctx, width, height, "总大小为 0");
    return [];
  }

  const rects: TreemapRect[] = [];

  const inset = 2;
  squarify(layoutItems, { x: inset, y: inset, w: width - inset * 2, h: height - inset * 2 }, total, 0, rects);

  for (const rect of rects) {
    drawRect(ctx, rect);
  }

  return rects;
}

function collectItems(nodes: NodeDto[], rootId: number): LayoutItem[] {
  const result: LayoutItem[] = [];
  const stack = [...nodes[rootId].children];

  while (stack.length > 0) {
    const id = stack.pop()!;
    const n = nodes[id];
    if (!n) continue;
    if (n.totalAllocated <= 0) continue;

    if (n.isDir && n.children.length > 0) {
      stack.push(...n.children);
    } else {
      result.push({ id: n.id, size: n.totalAllocated, node: n });
    }
  }

  return result;
}

function squarify(
  items: LayoutItem[],
  bounds: Bounds,
  total: number,
  depth: number,
  out: TreemapRect[]
) {
  if (items.length === 0 || bounds.w <= 0 || bounds.h <= 0 || total <= 0) return;

  const shortSide = Math.min(bounds.w, bounds.h);
  const area = bounds.w * bounds.h;

  let remaining = items.slice();
  let curBounds = { ...bounds };
  let curTotal = total;

  while (remaining.length > 0 && curBounds.w > 1 && curBounds.h > 1 && curTotal > 0) {
    const row: LayoutItem[] = [];
    let rowSum = 0;
    let bestRatio = Infinity;

    for (const item of remaining) {
      const candidateSum = rowSum + item.size;
      const ratio = worstRatio([...row, item], candidateSum, shortSide, curBounds, curTotal);
      if (ratio > bestRatio && row.length > 0) break;
      row.push(item);
      rowSum = candidateSum;
      bestRatio = ratio;
    }

    const rowFraction = rowSum / curTotal;
    const rowArea = area * rowFraction;
    const pixelBudget = (curBounds.w >= curBounds.h ? curBounds.w : curBounds.h);

    let x = curBounds.x;
    let y = curBounds.y;
    let w = curBounds.w;
    let h = curBounds.h;

    if (w >= h) {
      const availablePixels = pixelBudget;
      const minPixelSum = row.length * 2;
      const rowW = rowSum > 0 ? Math.max(2, Math.min(availablePixels - minPixelSum + row.length * 2, rowArea / h)) : 2;

      for (const item of row) {
        let itemH = item.size > 0 ? Math.max(2, (item.size / rowSum) * h) : 2;
        if (y + itemH > curBounds.y + curBounds.h) {
          itemH = Math.max(0, curBounds.y + curBounds.h - y);
        }
        out.push({ id: item.id, x, y, w: rowW, h: itemH, depth, node: item.node });
        y += itemH;
      }
      curBounds = { x: x + rowW, y: curBounds.y, w: Math.max(0, curBounds.w - rowW), h: curBounds.h };
    } else {
      const availablePixels = pixelBudget;
      const minPixelSum = row.length * 2;
      const rowH = rowSum > 0 ? Math.max(2, Math.min(availablePixels - minPixelSum + row.length * 2, rowArea / w)) : 2;

      for (const item of row) {
        let itemW = item.size > 0 ? Math.max(2, (item.size / rowSum) * w) : 2;
        if (x + itemW > curBounds.x + curBounds.w) {
          itemW = Math.max(0, curBounds.x + curBounds.w - x);
        }
        out.push({ id: item.id, x, y, w: itemW, h: rowH, depth, node: item.node });
        x += itemW;
      }
      curBounds = { x: curBounds.x, y: y + rowH, w: curBounds.w, h: Math.max(0, curBounds.h - rowH) };
    }

    remaining = remaining.slice(row.length);
    curTotal -= rowSum;
  }
}

function worstRatio(
  row: LayoutItem[],
  rowSum: number,
  shortSide: number,
  bounds: Bounds,
  total: number
): number {
  if (rowSum <= 0 || total <= 0) return Infinity;
  const totalArea = bounds.w * bounds.h;
  const rowArea = (rowSum / total) * totalArea;
  if (rowArea <= 0) return Infinity;

  const maxSize = Math.max(...row.map((i) => i.size));
  const minSize = Math.min(...row.map((i) => i.size));
  const maxA = (maxSize / rowSum) * rowArea;
  const minA = (minSize / rowSum) * rowArea;
  const s2 = shortSide * shortSide;

  return Math.max(
    (s2 * maxA) / (rowArea * rowArea),
    (rowArea * rowArea) / (s2 * minA)
  );
}

function drawRect(ctx: CanvasRenderingContext2D, rect: TreemapRect) {
  if (rect.w <= 0 || rect.h <= 0) return;

  const node = rect.node;
  let color: string;

  if (node.isDir) {
    const hue = hashHue(node.name);
    color = `hsl(${hue}, 35%, 35%)`;
  } else {
    const ext = node.extension || "";
    const hue = hashHue(ext || node.name);
    color = `hsl(${hue}, 55%, 50%)`;
  }

  ctx.fillStyle = color;
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);

  ctx.strokeStyle = "rgba(0, 0, 0, 0.35)";
  ctx.lineWidth = 0.5;
  ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);

  if (rect.w < 30 || rect.h < 16) return;

  ctx.save();
  ctx.beginPath();
  ctx.rect(rect.x + 2, rect.y + 2, rect.w - 4, rect.h - 4);
  ctx.clip();

  ctx.fillStyle = "rgba(255,255,255,0.9)";
  ctx.font = "bold 11px system-ui, -apple-system, sans-serif";
  ctx.fillText(clipText(ctx, node.name, rect.w - 8), rect.x + 3, rect.y + 13);

  if (rect.h >= 34) {
    ctx.fillStyle = "rgba(255,255,255,0.65)";
    ctx.font = "10px system-ui, -apple-system, sans-serif";
    ctx.fillText(formatBytes(node.totalAllocated), rect.x + 3, rect.y + 26);
  }

  ctx.restore();
}

function drawEmpty(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  message: string
) {
  ctx.fillStyle = "#161b22";
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "#8b949e";
  ctx.font = "14px system-ui, -apple-system, sans-serif";
  ctx.textAlign = "center";
  ctx.fillText(message, width / 2, height / 2);
  ctx.textAlign = "start";
}

export function hitTestTreemap(
  rects: TreemapRect[],
  x: number,
  y: number
): number | null {
  for (let i = rects.length - 1; i >= 0; i -= 1) {
    const r = rects[i];
    if (x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h) {
      return r.id;
    }
  }
  return null;
}

export function hitTestTreemapNode(
  rects: TreemapRect[],
  x: number,
  y: number
): TreemapRect | null {
  for (let i = rects.length - 1; i >= 0; i -= 1) {
    const r = rects[i];
    if (x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h) {
      return r;
    }
  }
  return null;
}

function clipText(ctx: CanvasRenderingContext2D, text: string, maxW: number): string {
  if (ctx.measureText(text).width <= maxW) return text;
  let t = text;
  while (t.length > 1 && ctx.measureText(t + "...").width > maxW) {
    t = t.slice(0, -1);
  }
  return t + "...";
}

function hashHue(text: string): number {
  let hash = 2166136261;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash) % 360;
}

export function buildNodePath(id: number, nodeMap: Map<number, NodeDto>, rootId: number): string {
  const parts: string[] = [];
  let current: number | null | undefined = id;
  while (current != null && nodeMap.has(current)) {
    const n = nodeMap.get(current) as NodeDto;
    parts.push(n.name);
    if (current === rootId) break;
    current = n.parent;
  }
  parts.reverse();
  return parts.join("\\");
}
