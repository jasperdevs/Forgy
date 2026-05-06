#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const exe = process.platform === "win32" ? "forge.exe" : "forge";
const binary = process.env.FORGY_FORGE_BIN || path.join(__dirname, "..", "vendor", exe);

if (!fs.existsSync(binary)) {
  console.error("Forgy could not find the forge binary.");
  console.error("Try reinstalling with: npm install -g forgy");
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 0);
