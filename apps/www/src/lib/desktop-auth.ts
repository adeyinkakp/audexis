import { randomBytes } from "node:crypto";
import { and, eq, gt, lt } from "drizzle-orm";
import { db } from "../db";
import { apiKey, user, verification } from "../db/schema";
import {
  hashSecret,
  hasCloudSubscription,
  loopbackRedirect,
  validChallenge,
  verifyPkce,
} from "./desktop-auth-protocol";

const codeNamespace = "audexis-desktop:";
export async function issueDesktopCode(
  userId: string,
  redirectUri: string,
  challenge: string,
) {
  const code = randomBytes(32).toString("base64url");
  await db.transaction(async (tx) => {
    await tx
      .delete(verification)
      .where(
        and(
          eq(verification.identifier, codeNamespace + userId),
          lt(verification.expiresAt, new Date()),
        ),
      );
    await tx.insert(verification).values({
      id: codeNamespace + hashSecret(code),
      identifier: codeNamespace + userId,
      value: JSON.stringify({ userId, redirectUri, challenge }),
      expiresAt: new Date(Date.now() + 60_000),
      createdAt: new Date(),
    });
  });
  return code;
}

export async function redeemDesktopCode(
  code: unknown,
  verifier: unknown,
  redirect: unknown,
) {
  if (
    typeof code !== "string" ||
    !validChallenge(code) ||
    !loopbackRedirect(redirect)
  )
    return null;
  return db.transaction(async (tx) => {
    const [record] = await tx
      .select()
      .from(verification)
      .where(
        and(
          eq(verification.id, codeNamespace + hashSecret(code)),
          gt(verification.expiresAt, new Date()),
        ),
      )
      .for("update");
    if (!record || record.expiresAt.getTime() <= Date.now()) return null;
    const grant = JSON.parse(record.value) as {
      userId: string;
      redirectUri: string;
      challenge: string;
    };
    if (
      record.identifier !== codeNamespace + grant.userId ||
      grant.redirectUri !== redirect ||
      !verifyPkce(verifier, grant.challenge)
    )
      return null;
    await tx.delete(verification).where(eq(verification.id, record.id));
    const [subscriber] = await tx
      .select()
      .from(user)
      .where(eq(user.id, grant.userId))
      .for("update");
    if (!hasCloudSubscription(subscriber)) return null;
    const token = `audexis_cloud_${randomBytes(24).toString("hex")}`;
    await tx.insert(apiKey).values({
      id: `key_${randomBytes(12).toString("hex")}`,
      userId: subscriber.id,
      keyHash: hashSecret(token),
      label: "Audexis Desktop App",
      createdAt: new Date(),
    });
    return token;
  });
}

export async function desktopCredential(token: string) {
  if (!/^audexis_cloud_[a-f0-9]{48}$/.test(token)) return null;
  const [record] = await db
    .select({ key: apiKey, subscriber: user })
    .from(apiKey)
    .innerJoin(user, eq(apiKey.userId, user.id))
    .where(eq(apiKey.keyHash, hashSecret(token)));
  return record ?? null;
}

export function bearerToken(request: Request): string | null {
  const value = request.headers.get("authorization");
  return value?.match(/^Bearer (audexis_cloud_[a-f0-9]{48})$/)?.[1] ?? null;
}
