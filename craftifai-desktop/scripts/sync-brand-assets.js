/**
 * Sync FirmGen brand assets from the single source logo (assets/new_logo.png)
 * into every location the desktop app and GUI build expect.
 *
 * Run automatically before GUI/electron builds via package.json.
 */
const fs = require("fs");
const path = require("path");

const ASSETS = path.join(__dirname, "..", "assets");
const GUI_PUBLIC = path.join(__dirname, "..", "..", "refact-agent", "gui", "public");
const SOURCE = path.join(ASSETS, "new_logo.png");

const COPY_TARGETS = [
  path.join(GUI_PUBLIC, "new_logo.png"),
  path.join(GUI_PUBLIC, "favicon.png"),
  path.join(ASSETS, "icon.png"),
  path.join(ASSETS, "tray-icon.png"),
];

function copyFile(from, to) {
  fs.mkdirSync(path.dirname(to), { recursive: true });
  fs.copyFileSync(from, to);
  console.log(`  synced ${path.relative(path.join(__dirname, "..", ".."), to)}`);
}

async function writeIco() {
  const { default: pngToIco } = await import("png-to-ico");
  const buf = await pngToIco(SOURCE);
  const icoPath = path.join(ASSETS, "icon.ico");
  fs.writeFileSync(icoPath, buf);
  console.log(`  synced ${path.relative(path.join(__dirname, "..", ".."), icoPath)}`);
}

async function main() {
  if (!fs.existsSync(SOURCE)) {
    console.error(`ERROR: Brand logo not found at ${SOURCE}`);
    process.exit(1);
  }

  console.log("Syncing brand assets from craftifai-desktop/assets/new_logo.png …");
  for (const target of COPY_TARGETS) {
    copyFile(SOURCE, target);
  }

  await writeIco();
}

main().catch((err) => {
  console.error("ERROR:", err.message);
  process.exit(1);
});
