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
  includeSystemFiles?: boolean;
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
