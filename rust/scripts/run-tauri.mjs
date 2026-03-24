import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rustDir = path.resolve(__dirname, "..");
const srcTauriDir = path.join(rustDir, "src-tauri");

function run(command, args, cwd, captureStdout = false) {
  return new Promise((resolve, reject) => {
    let stdout = "";
    const child = spawn(command, args, {
      cwd,
      stdio: captureStdout ? ["inherit", "pipe", "inherit"] : "inherit",
      shell: false,
    });

    if (captureStdout && child.stdout) {
      child.stdout.on("data", (chunk) => {
        const text = chunk.toString();
        stdout += text;
        process.stdout.write(text);
      });
    }

    child.on("exit", (code) => {
      if (code === 0) {
        resolve(stdout);
      } else {
        reject(new Error(`${command} ${args.join(" ")} failed with exit code ${code}`));
      }
    });

    child.on("error", reject);
  });
}

const args = process.argv.slice(2);
let finalArgs = [...args];
const isBuild = args[0] === "build";
const hasExplicitTarget = args.includes("--target");
const hasNoSignFlag = args.includes("--no-sign");

if (
  isBuild &&
  !hasExplicitTarget &&
  args[1] &&
  args[1].includes("-") &&
  !args[1].startsWith("-")
) {
  finalArgs = [args[0], "--target", args[1], ...args.slice(2)];
}

if (
  isBuild &&
  !hasNoSignFlag &&
  !process.env.TAURI_SIGNING_PRIVATE_KEY &&
  !process.env.WINDOWS_CERTIFICATE_THUMBPRINT &&
  !process.env.APPLE_SIGNING_IDENTITY
) {
  finalArgs = [...finalArgs, "--no-sign"];
}

if (process.platform === "win32" && isBuild) {
  const output = await run(
    "node",
    [path.join("scripts", "prepare-windows-bundle.mjs")],
    rustDir,
    true,
  );
  const generatedConfigPath = output
    .trim()
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .at(-1);

  if (!generatedConfigPath) {
    throw new Error("prepare-windows-bundle did not produce a generated config path");
  }

  finalArgs = [...finalArgs, "--config", generatedConfigPath];
}

if (process.platform === "win32") {
  const tauriExecutable = path.join(rustDir, "node_modules", ".bin", "tauri.cmd");
  await run("cmd", ["/c", tauriExecutable, ...finalArgs], rustDir);
} else {
  const tauriExecutable = path.join(rustDir, "node_modules", ".bin", "tauri");
  await run(tauriExecutable, finalArgs, rustDir);
}
