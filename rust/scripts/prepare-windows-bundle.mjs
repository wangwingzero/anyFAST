import { mkdir, copyFile, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rustDir = path.resolve(__dirname, "..");
const srcTauriDir = path.join(rustDir, "src-tauri");
const binariesDir = path.join(srcTauriDir, "binaries");
const serviceSource = path.join(srcTauriDir, "target", "release", "anyfast-service.exe");
const bundledService = path.join(
  binariesDir,
  "anyfast-service-x86_64-pc-windows-msvc.exe",
);
const generatedConfig = path.join(
  srcTauriDir,
  "target",
  "tauri.windows.generated.conf.json",
);

function run(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      stdio: "inherit",
      shell: false,
    });

    child.on("exit", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} ${args.join(" ")} failed with exit code ${code}`));
      }
    });

    child.on("error", reject);
  });
}

async function main() {
  if (process.platform !== "win32") {
    return;
  }

  await run("cargo", ["build", "--release", "--bin", "anyfast-service"], srcTauriDir);

  await mkdir(binariesDir, { recursive: true });
  await copyFile(serviceSource, bundledService);

  const configPath = path.join(srcTauriDir, "tauri.conf.json");
  const config = JSON.parse(await readFile(configPath, "utf8"));
  const bundle = config.bundle ?? {};
  const existingResources = bundle.resources ?? {};
  const resourceEntries = Array.isArray(existingResources)
    ? existingResources
    : { ...existingResources };

  if (Array.isArray(resourceEntries)) {
    if (!resourceEntries.includes("binaries/anyfast-service-x86_64-pc-windows-msvc.exe")) {
      resourceEntries.push("binaries/anyfast-service-x86_64-pc-windows-msvc.exe");
    }
  } else {
    resourceEntries["binaries/anyfast-service-x86_64-pc-windows-msvc.exe"] =
      "anyfast-service.exe";
  }

  config.bundle = {
    ...bundle,
    resources: resourceEntries,
  };

  await mkdir(path.dirname(generatedConfig), { recursive: true });
  await writeFile(generatedConfig, `${JSON.stringify(config, null, 2)}\n`, "utf8");

  process.stdout.write(`${generatedConfig}\n`);
}

await main();
