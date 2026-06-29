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

export interface ExtensionStat {
  extension: string;
  size: number;
  allocated: number;
  fileCount: number;
}

export type ExtCategory = "document" | "image" | "video" | "audio" | "archive" | "code" | "executable" | "other";

export const EXTENSION_CATEGORIES: Record<string, ExtCategory> = {
  // 文档类
  doc: "document", docx: "document", xls: "document", xlsx: "document",
  ppt: "document", pptx: "document", pdf: "document", txt: "document",
  rtf: "document", csv: "document", odt: "document", ods: "document",
  odp: "document", wps: "document", wpt: "document", pages: "document",
  numbers: "document", key: "document", dot: "document", dotx: "document",
  xlt: "document", xltx: "document", pot: "document", potx: "document",
  pps: "document", ppsx: "document",

  // 图片类
  jpg: "image", jpeg: "image", png: "image", gif: "image",
  bmp: "image", webp: "image", svg: "image", ico: "image",
  tif: "image", tiff: "image", raw: "image", cr2: "image",
  nef: "image", arw: "image", dng: "image", psd: "image",
  ai: "image", eps: "image", heic: "image", heif: "image",
  avif: "image", jp2: "image",

  // 视频类
  mp4: "video", avi: "video", mkv: "video", mov: "video",
  wmv: "video", flv: "video", webm: "video", mpg: "video",
  mpeg: "video", m4v: "video", m2t: "video", rmvb: "video",
  rm: "video", vob: "video", ogv: "video", "3gp": "video",
  mts: "video", m2ts: "video",

  // 音频类
  mp3: "audio", wav: "audio", flac: "audio", ogg: "audio",
  m4a: "audio", wma: "audio", aac: "audio", ape: "audio",
  dsd: "audio", dff: "audio", aiff: "audio", mid: "audio",
  midi: "audio", opus: "audio", ra: "audio",

  // 压缩包
  zip: "archive", rar: "archive", "7z": "archive", tar: "archive",
  gz: "archive", bz2: "archive", xz: "archive", iso: "archive",
  zst: "archive", lz: "archive", lzma: "archive", cab: "archive",
  arj: "archive", deb: "archive", rpm: "archive", dmg: "archive",
  pkg: "archive",

  // 代码类
  js: "code", ts: "code", py: "code", java: "code", cpp: "code",
  c: "code", h: "code", rs: "code", go: "code", rb: "code",
  php: "code", html: "code", css: "code", json: "code", xml: "code",
  yaml: "code", toml: "code", vue: "code", svelte: "code",
  swift: "code", kt: "code", scala: "code", sh: "code", bash: "code",
  ps1: "code", bat: "code", cmd: "code", sql: "code", r: "code",
  m: "code", mm: "code", pl: "code", pm: "code", lua: "code",
  dart: "code", jsx: "code", tsx: "code", cfg: "code", ini: "code",
  conf: "code", env: "code", makefile: "code", dockerfile: "code",
  cs: "code", fs: "code", hs: "code", clj: "code", groovy: "code",
  gradle: "code", cmake: "code", zig: "code", nim: "code",

  // 可执行文件
  exe: "executable", dll: "executable", msi: "executable",
  com: "executable", sys: "executable", drv: "executable",
  scr: "executable", cpl: "executable",
};

export function extCategory(ext: string | null): ExtCategory {
  if (!ext) return "other";
  return EXTENSION_CATEGORIES[ext.toLowerCase()] ?? "other";
}

export const CATEGORY_COLORS: Record<ExtCategory, string> = {
  document: "hsl(210, 60%, 50%)",
  image: "hsl(120, 55%, 45%)",
  video: "hsl(0, 60%, 50%)",
  audio: "hsl(30, 70%, 50%)",
  archive: "hsl(280, 50%, 50%)",
  code: "hsl(190, 60%, 45%)",
  executable: "hsl(350, 55%, 50%)",
  other: "hsl(0, 0%, 45%)",
};

export const CATEGORY_LABELS: Record<ExtCategory, string> = {
  document: "文档",
  image: "图片",
  video: "视频",
  audio: "音频",
  archive: "压缩包",
  code: "代码",
  executable: "可执行文件",
  other: "其他",
};
