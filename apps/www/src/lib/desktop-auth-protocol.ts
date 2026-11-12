import { createHash, timingSafeEqual } from "node:crypto";

export const privateHeaders = {
  "Cache-Control": "no-store",
  Pragma: "no-cache",
  "Referrer-Policy": "no-referrer",
  "X-Content-Type-Options": "nosniff",
};

export function hashSecret(value: string): string {
  return createHash("sha256").update(value).digest("hex");
}

export function validChallenge(value: unknown): value is string {
  return typeof value === "string" && /^[A-Za-z0-9_-]{43}$/.test(value);
}

export function validState(value: unknown): value is string {
  return typeof value === "string" && /^[A-Za-z0-9_-]{43,128}$/.test(value);
}

export function verifyPkce(verifier: unknown, challenge: string): boolean {
  if (
    typeof verifier !== "string" ||
    !/^[A-Za-z0-9._~-]{43,128}$/.test(verifier)
  )
    return false;
  const actual = createHash("sha256").update(verifier).digest("base64url");
  return (
    validChallenge(challenge) &&
    timingSafeEqual(Buffer.from(actual), Buffer.from(challenge))
  );
}

export function loopbackRedirect(value: unknown): string | null {
  if (typeof value !== "string" || value.length > 256) return null;
  try {
    const url = new URL(value);
    if (
      url.protocol !== "http:" ||
      url.hostname !== "127.0.0.1" ||
      !url.port ||
      Number(url.port) < 1024 ||
      Number(url.port) > 65535 ||
      url.username ||
      url.password ||
      url.pathname !== "/callback" ||
      url.search ||
      url.hash
    )
      return null;
    return url.href;
  } catch {
    return null;
  }
}

export function nativeRedirect(value: unknown): string | null {
  if (value === "audexis://callback") return value;
  return loopbackRedirect(value);
}

export function safeLoginCallback(
  value: string | null,
  origin: string,
): string {
  if (!value || !value.startsWith("/") || value.startsWith("//"))
    return "/dashboard";
  try {
    const url = new URL(value, origin);
    if (
      url.origin !== origin ||
      !["/auth/desktop", "/dashboard"].includes(url.pathname)
    )
      return "/dashboard";
    return url.pathname + url.search;
  } catch {
    return "/dashboard";
  }
}

export function hasCloudSubscription(
  user:
    | {
        subscriptionStatus: string | null;
        currentPeriodEnd: Date | null;
      }
    | null
    | undefined,
  now = Date.now(),
): boolean {
  return (
    !!user &&
    ["active", "trialing"].includes(user.subscriptionStatus ?? "") &&
    !!user.currentPeriodEnd &&
    user.currentPeriodEnd.getTime() > now
  );
}

export function sameOriginPost(request: Request): boolean {
  return request.headers.get("origin") === new URL(request.url).origin;
}

export function validAuthorization(params: URLSearchParams) {
  const redirectUri = nativeRedirect(params.get("redirect_uri"));
  const state = params.get("state");
  const challenge = params.get("code_challenge");
  const keys = [
    "redirect_uri",
    "state",
    "code_challenge",
    "code_challenge_method",
  ];
  if (
    !redirectUri ||
    !validState(state) ||
    !validChallenge(challenge) ||
    params.get("code_challenge_method") !== "S256" ||
    keys.some((key) => params.getAll(key).length !== 1)
  )
    return null;
  return { redirectUri, state, challenge };
}

export function desktopErrorRedirect(
  callback: string | null,
  origin: string,
  error: string | null,
): string | null {
  const safeCallback = safeLoginCallback(callback, origin);
  const url = new URL(safeCallback, origin);
  if (url.pathname !== "/auth/desktop") return null;
  const grant = validAuthorization(url.searchParams);
  if (!grant) return null;
  const target = new URL(grant.redirectUri);
  target.searchParams.set("state", grant.state);
  target.searchParams.set(
    "error",
    error === "access_denied" ? "access_denied" : "oauth_error",
  );
  return target.href;
}
