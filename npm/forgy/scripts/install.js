const fs = require("node:fs");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const vendor = path.join(root, "vendor");
const pkg = require(path.join(root, "package.json"));
const version = pkg.version;

function platformAsset() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "win32" && arch === "x64") return ["forge-windows-x64.zip", "forge.exe"];
  if (platform === "linux" && arch === "x64") return ["forge-linux-x64.tar.gz", "forge"];
  if (platform === "darwin" && arch === "x64") return ["forge-macos-x64.tar.gz", "forge"];
  if (platform === "darwin" && arch === "arm64") return ["forge-macos-arm64.tar.gz", "forge"];

  throw new Error(`Unsupported platform: ${platform} ${arch}`);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { stdio: "inherit", ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

function copyLocal(binaryName) {
  const local = process.env.FORGY_LOCAL_BINARY;
  if (!local) return false;
  fs.mkdirSync(vendor, { recursive: true });
  fs.copyFileSync(local, path.join(vendor, binaryName));
  return true;
}

function download(url, destination) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destination);
    https
      .get(url, (response) => {
        if ([301, 302, 303, 307, 308].includes(response.statusCode) && response.headers.location) {
          file.close();
          fs.unlinkSync(destination);
          download(response.headers.location, destination).then(resolve, reject);
          return;
        }
        if (response.statusCode !== 200) {
          file.close();
          fs.unlinkSync(destination);
          reject(new Error(`Download failed: ${response.statusCode} ${response.statusMessage}`));
          return;
        }
        response.pipe(file);
        file.on("finish", () => file.close(resolve));
      })
      .on("error", (error) => {
        file.close();
        if (fs.existsSync(destination)) fs.unlinkSync(destination);
        reject(error);
      });
  });
}

async function main() {
  const [asset, binaryName] = platformAsset();
  if (copyLocal(binaryName)) return;

  fs.mkdirSync(vendor, { recursive: true });
  const tmp = path.join(os.tmpdir(), asset);
  const url = `https://github.com/jasperdevs/Forgy/releases/download/v${version}/${asset}`;

  await download(url, tmp);

  if (asset.endsWith(".zip")) {
    run("powershell", [
      "-NoProfile",
      "-ExecutionPolicy",
      "Bypass",
      "-Command",
      `Expand-Archive -LiteralPath '${tmp.replace(/'/g, "''")}' -DestinationPath '${vendor.replace(/'/g, "''")}' -Force`,
    ]);
  } else {
    run("tar", ["-xzf", tmp, "-C", vendor]);
  }

  const binary = path.join(vendor, binaryName);
  if (!fs.existsSync(binary)) throw new Error(`Downloaded archive did not contain ${binaryName}`);
  if (process.platform !== "win32") fs.chmodSync(binary, 0o755);
}

main().catch((error) => {
  console.error(`Failed to install Forgy: ${error.message}`);
  console.error("Install from source instead: cargo install --git https://github.com/jasperdevs/Forgy forge-cli");
  process.exit(1);
});
