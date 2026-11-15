export const linuxTargets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
];

export function releaseAssets(files) {
  return [
    [
      /\.app\.tar\.gz$/,
      ["macos-aarch64", "macos-x86_64", "darwin-aarch64", "darwin-x86_64"],
      "app",
    ],
    [/\.exe$/, ["windows-x86_64"], "nsis"],
    [/(?:_|-)(?:x86_64|amd64)\.AppImage$/, ["linux-x86_64"], "appimage"],
    [/(?:_|-)(?:aarch64|arm64)\.AppImage$/, ["linux-aarch64"], "appimage"],
  ].map(([pattern, names, format]) => {
    const matches = files.filter((name) => pattern.test(name));
    if (matches.length !== 1) {
      throw new Error(
        `Expected exactly one release asset for ${names.join(", ")}; found ${matches.length}`,
      );
    }
    return { file: matches[0], names, format };
  });
}
