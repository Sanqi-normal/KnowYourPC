const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "..");
const profile = process.argv[2] || "debug";
const isWindows = process.platform === "win32";
const ext = isWindows ? ".exe" : "";

const src = path.join(root, "target", profile, `fastscan-mcp${ext}`);
const destDir = path.join(root, "src-tauri", "binaries");
const dest = path.join(destDir, `fastscan-mcp${ext}`);

if (!fs.existsSync(src)) {
  console.error(`Source not found: ${src}`);
  process.exit(1);
}

fs.mkdirSync(destDir, { recursive: true });
fs.copyFileSync(src, dest);
console.log(`Copied ${src} -> ${dest}`);
