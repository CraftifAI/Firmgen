const fs = require("fs");
const path = require("path");

const outputRoot = path.join(__dirname, "..", "dist-packages");
const winUnpacked = path.join(outputRoot, "win-unpacked");
const asarPath = path.join(winUnpacked, "resources", "app.asar");

function isLocked(filePath) {
  if (!fs.existsSync(filePath)) {
    return false;
  }
  try {
    const fd = fs.openSync(filePath, "r+");
    fs.closeSync(fd);
    return false;
  } catch {
    return true;
  }
}

function tryRemoveDir(dirPath) {
  try {
    fs.rmSync(dirPath, { recursive: true, force: true });
    return true;
  } catch {
    return false;
  }
}

if (!fs.existsSync(winUnpacked)) {
  process.exit(0);
}

if (tryRemoveDir(winUnpacked)) {
  process.exit(0);
}

if (isLocked(asarPath)) {
  console.error("");
  console.error("ERROR: dist-packages\\win-unpacked is locked by another process.");
  console.error("electron-builder cannot replace resources\\app.asar.");
  console.error("");
  console.error("Common cause: Cursor (or VS Code) has dist-packages files open or watched.");
  console.error("");
  console.error("Fix options:");
  console.error("  1. Close any editor tabs under craftifai-desktop\\dist-packages\\");
  console.error("  2. Close the running CraftifAI app if it is open");
  console.error("  3. In Cursor: Developer: Reload Window, then rebuild");
  console.error("  4. Build to a fresh output folder instead:");
  console.error('       npx electron-builder --win portable "--config.directories.output=dist-out"');
  console.error("");
  process.exit(1);
}

console.error("");
console.error("ERROR: Could not clean dist-packages\\win-unpacked before packaging.");
console.error("Close apps using that folder and retry.");
console.error("");
process.exit(1);
