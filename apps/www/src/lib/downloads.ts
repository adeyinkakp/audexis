export const releasesUrl = "https://github.com/kp-fyn/audexis/releases/latest";
export const latestReleaseApi = "https://api.github.com/repos/kp-fyn/audexis/releases/latest";

interface ReleaseAsset {
  name: string;
  browser_download_url: string;
  size: number;
}

export function findInstaller(assets: unknown, platform: string): ReleaseAsset | undefined {
  if (!Array.isArray(assets)) return;
  return assets.find((asset): asset is ReleaseAsset => {
    if (!asset || typeof asset.name !== "string" ||
        typeof asset.browser_download_url !== "string" ||
        typeof asset.size !== "number") return false;
    const name = asset.name.toLowerCase();
    const matches = platform === "macos"
      ? name.endsWith(".dmg") && name.includes("universal")
      : platform === "windows" && name.endsWith(".exe") &&
        /(?:x64|x86_64|amd64)/.test(name) && !/(?:arm64|aarch64)/.test(name);
    if (!matches) return false;
    try {
      const url = new URL(asset.browser_download_url);
      return url.protocol === "https:" && url.hostname === "github.com" &&
        /^\/(?:kp-fyn|adeyinkakp)\/audexis\/releases\/download\//.test(url.pathname);
    } catch {
      return false;
    }
  });
}
