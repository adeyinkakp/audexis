// shout out gtt
import { execFileSync } from "node:child_process";
import {
  readFileSync,
  writeFileSync,
  appendFileSync,
  readdirSync,
} from "node:fs";
import { dirname, resolve, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const config = JSON.parse(
  readFileSync(join(root, "apps/desktop/Packager.json"), "utf8"),
);
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    ["metadata", "--no-deps", "--format-version", "1", "--offline"],
    {
      cwd: root,
      encoding: "utf8",
    },
  ),
);
const version = metadata.packages.find((pkg) => pkg.name === "audexis").version;
const tag = `v${version}`;
if (
  config.version !== version ||
  process.env.GITHUB_REF !== `refs/tags/${tag}`
) {
  throw new Error(
    "Run releases from the version tag matching Cargo.toml and Packager.json",
  );
}

if (process.argv[2] === "configure") {
  const target = process.argv[3];
  const required = ["CARGO_PACKAGER_SIGN_PRIVATE_KEY"];
  if (target === "universal-apple-darwin") {
    required.push(
      "APPLE_SIGNING_IDENTITY",
      "APPLE_ID",
      "APPLE_PASSWORD",
      "APPLE_TEAM_ID",
    );
    config.macos = {
      ...config.macos,
      signingIdentity: process.env.APPLE_SIGNING_IDENTITY,
    };
    config.formats = ["app", "dmg"];
  } else if (target === "x86_64-pc-windows-msvc") {
    required.push(
      "AZURE_CLIENT_ID",
      "AZURE_TENANT_ID",
      "AZURE_CLIENT_SECRET",
      "AZURE_ENDPOINT",
      "AZURE_CODE_SIGNING_ACCOUNT",
      "AZURE_CERTIFICATE_PROFILE_NAME",
    );
    config.formats = ["nsis"];
  } else {
    throw new Error(`Unsupported release target: ${target}`);
  }
  const missing = required.filter((name) => !process.env[name]);
  if (missing.length)
    throw new Error(`Missing release credentials: ${missing.join(", ")}`);
  const desktop = join(root, "apps/desktop");
  config.icons = config.icons.map((icon) => resolve(desktop, icon));
  if (config.nsis?.installerIcon)
    config.nsis.installerIcon = resolve(desktop, config.nsis.installerIcon);
  delete config.beforePackagingCommand;
  config.outDir = join(root, "target/packages");
  config.binariesDir = join(root, "target", target, "release");
  config.targetTriple = target;
  writeFileSync(
    join(process.env.RUNNER_TEMP, "Packager.json"),
    JSON.stringify(config),
  );
} else if (process.argv[2] === "manifest") {
  const assets = join(root, "target/release-assets");
  const files = readdirSync(assets);
  const platforms = {};
  const base = `https://github.com/${process.env.GITHUB_REPOSITORY}/releases/download/${tag}/`;
  for (const [suffix, names] of [
    [".app.tar.gz", ["darwin-aarch64", "darwin-x86_64"]],
    [".exe", ["windows-x86_64"]],
  ]) {
    const matches = files.filter((name) => name.endsWith(suffix));
    if (matches.length !== 1)
      throw new Error(`Expected exactly one release asset ending in ${suffix}`);
    const file = matches[0];
    const signature = readFileSync(join(assets, `${file}.sig`), "utf8").trim();
    for (const name of names)
      platforms[name] = { signature, url: base + encodeURIComponent(file) };
  }
  const manifest = {
    version,
    notes: `See https://www.audexis.app/releases/${tag}`,
    pub_date: new Date().toISOString(),
    platforms,
  };
  writeFileSync(
    join(assets, "latest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  appendFileSync(process.env.GITHUB_OUTPUT, `tag=${tag}\nversion=${version}\n`);
} else {
  throw new Error("Unknown release command");
}
