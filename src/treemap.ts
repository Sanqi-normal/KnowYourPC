import type { NodeDto } from "./types";
import { formatBytes } from "./format";

export interface TreemapRect {
  id: number;
  x: number;
  y: number;
  w: number;
  h: number;
  depth: number;
}

const MAX_CHILDREN_PER_LEVEL = 96;
const MAX_DEPTH = 5;

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
    drawEmpty(ctx, width, height, "\u6682\u65E0\u6570\u636E");
    return [];
  }

  if (!root.children.length || root.totalAllocated <= 0) {
    drawEmpty(ctx, width, height, "\u6B64\u8282\u70B9\u6CA1\u6709\u53EF\u89C6\u5316\u5B50\u9879");
    return [];
  }

  const rects: TreemapRect[] = [];

  layoutSliceDice(
    nodes,
    rootId,
    0,
    0,
    width,
    height,
    0,
    true,
    rects
  );

  for (const rect of rects) {
    drawRect(ctx, nodes[rect.id], rect);
  }

  return rects;
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

function layoutSliceDice(
  nodes: NodeDto[],
  parentId: number,
  x: number,
  y: number,
  w: number,
  h: number,
  depth: number,
  horizontal: boolean,
  out: TreemapRect[]
): void {
  if (depth >= MAX_DEPTH || w < 8 || h < 8) return;

  const parent = nodes[parentId];
  if (!parent) return;

  const children = parent.children
    .map((id) => nodes[id])
    .filter((node): node is NodeDto => !!node && node.totalAllocated > 0)
    .slice(0, MAX_CHILDREN_PER_LEVEL);

  const total = children.reduce((sum, child) => sum + child.totalAllocated, 0);
  if (total <= 0) return;

  let cursor = horizontal ? x : y;

  for (const child of children) {
    const ratio = child.totalAllocated / total;
    const cw = horizontal ? w * ratio : w;
    const ch = horizontal ? h : h * ratio;

    const rect: TreemapRect = {
      id: child.id,
      x: horizontal ? cursor : x,
      y: horizontal ? y : cursor,
      w: Math.max(0, cw),
      h: Math.max(0, ch),
      depth
    };

    if (rect.w >= 2 && rect.h >= 2) {
      out.push(rect);

      const pad = depth < 2 ? 3 : 2;
      if (
        child.isDir &&
        child.children.length > 0 &&
        rect.w > 44 &&
        rect.h > 32
      ) {
        layoutSliceDice(
          nodes,
          child.id,
          rect.x + pad,
          rect.y + pad,
          Math.max(0, rect.w - pad * 2),
          Math.max(0, rect.h - pad * 2),
          depth + 1,
          !horizontal,
          out
        );
      }
    }

    cursor += horizontal ? cw : ch;
  }
}

function drawRect(
  ctx: CanvasRenderingContext2D,
  node: NodeDto,
  rect: TreemapRect
): void {
  const hue = hashHue(node.extension ?? node.name);
  const saturation = node.isDir ? 58 : 70;
  const lightness = Math.max(30, 52 - rect.depth * 4);

  ctx.fillStyle = `hsl(${hue}, ${saturation}%, ${lightness}%)`;
  ctx.fillRect(rect.x, rect.y, rect.w, rect.h);

  ctx.strokeStyle = "rgba(12, 18, 32, 0.75)";
  ctx.lineWidth = 1;
  ctx.strokeRect(rect.x + 0.5, rect.y + 0.5, rect.w - 1, rect.h - 1);

  if (rect.w < 72 || rect.h < 28) return;

  ctx.save();
  ctx.beginPath();
  ctx.rect(rect.x + 3, rect.y + 3, rect.w - 6, rect.h - 6);
  ctx.clip();

  ctx.fillStyle = "rgba(255,255,255,0.94)";
  ctx.font = "12px system-ui, -apple-system, BlinkMacSystemFont, Segoe UI";
  ctx.fillText(node.name, rect.x + 7, rect.y + 17);

  if (rect.h >= 46) {
    ctx.fillStyle = "rgba(255,255,255,0.72)";
    ctx.font = "11px system-ui, -apple-system, BlinkMacSystemFont, Segoe UI";
    ctx.fillText(formatBytes(node.totalAllocated), rect.x + 7, rect.y + 34);
  }

  ctx.restore();
}

function drawEmpty(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  message: string
): void {
  ctx.fillStyle = "#101827";
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "#9ca3af";
  ctx.font = "14px system-ui, -apple-system, BlinkMacSystemFont, Segoe UI";
  ctx.textAlign = "center";
  ctx.fillText(message, width / 2, height / 2);
  ctx.textAlign = "start";
}

function hashHue(text: string): number {
  let hash = 2166136261;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash) % 360;
}
