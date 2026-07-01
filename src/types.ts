export interface VolumeInfo {
  root: string;
  displayName: string;
  fsName: string;
  driveType: string;
  totalBytes: number;
  availableBytes: number;
  clusterSize: number;
  ntfsCandidate: boolean;
}

export type ScanMode = "auto" | "ntfsMft" | "walk";

export interface ScanOptions {
  root: string;
  mode: ScanMode;
}

export interface NodeDto {
  id: number;
  parent: number | null;
  name: string;
  isDir: boolean;
  size: number;
  allocated: number;
  totalSize: number;
  totalAllocated: number;
  childCount: number;
  children: number[];
  fileCount: number;
  dirCount: number;
  extension: string | null;
}

export interface ScanResult {
  root: string;
  scanner: string;
  elapsedMs: number;
  nodeCount: number;
  fileCount: number;
  dirCount: number;
  totalSize: number;
  totalAllocated: number;
  nodes: NodeDto[];
  warnings: string[];
}

export interface ProgressEvent {
  phase: string;
  processed: number;
  total: number | null;
  message: string;
}

export interface ExtensionStat {
  extension: string;
  size: number;
  allocated: number;
  fileCount: number;
}

export interface ChildNode {
  id: number;
  parent: number | null;
  name: string;
  isDir: boolean;
  size: number;
  allocated: number;
  totalSize: number;
  totalAllocated: number;
  childCount: number;
  fileCount: number;
  dirCount: number;
  extension: string | null;
}

export interface SearchResult {
  id: number;
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  allocated: number;
  totalSize: number;
  totalAllocated: number;
  extension: string | null;
}

export interface TreemapItem {
  id: number;
  size: number;
  name: string;
  isDir: boolean;
  extension: string | null;
  children?: TreemapItem[];
}

export const DIR_COLORS: string[] = [
  "hsl(0, 30%, 22%)",
  "hsl(25, 30%, 22%)",
  "hsl(50, 25%, 20%)",
  "hsl(140, 25%, 20%)",
  "hsl(175, 25%, 20%)",
  "hsl(210, 30%, 22%)",
  "hsl(240, 25%, 22%)",
  "hsl(280, 25%, 22%)",
  "hsl(320, 25%, 22%)",
  "hsl(10, 25%, 20%)",
  "hsl(45, 20%, 18%)",
  "hsl(195, 25%, 20%)",
];

export function dirColor(name: string): string {
  let hash = 2166136261;
  for (let i = 0; i < name.length; i += 1) {
    hash ^= name.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return DIR_COLORS[Math.abs(hash) % DIR_COLORS.length];
}
