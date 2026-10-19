import type { APIRoute } from "astro";
import { randomBytes, createHash } from "node:crypto";
import { db } from "../../db";
import { user as userTable, apiKey as apiKeyTable } from "../../db/schema";
import { eq } from "drizzle-orm";

export const POST: APIRoute = async ({ request, locals }) => {
  try {
    const sessionUser = locals.user;
    if (!sessionUser) {
      return new Response(JSON.stringify({ error: "Unauthorized" }), {
        status: 401,
        headers: { "Content-Type": "application/json" },
      });
    }

    const body = await request.json().catch(() => ({}));
    const parsedLabel = typeof body.label === "string" ? body.label.trim() : "";
    const label = parsedLabel.length > 0 ? parsedLabel : "Default Device";

    const [dbUser] = await db
      .select()
      .from(userTable)
      .where(eq(userTable.id, sessionUser.id));

    if (!dbUser) {
      return new Response(JSON.stringify({ error: "User not found" }), {
        status: 404,
        headers: { "Content-Type": "application/json" },
      });
    }

    const hasActiveSubscription =
      dbUser.subscriptionStatus === "active" ||
      dbUser.subscriptionStatus === "trialing";

    if (!hasActiveSubscription) {
      return new Response(
        JSON.stringify({
          error: "An active subscription is required to connect devices.",
          code: "PAYMENT_REQUIRED",
        }),
        {
          status: 402,
          headers: { "Content-Type": "application/json" },
        },
      );
    }

    const rawKey = `audexis_cloud_${randomBytes(24).toString("hex")}`;
    const keyHash = createHash("sha256").update(rawKey).digest("hex");
    const keyId = `key_${randomBytes(12).toString("hex")}`;

    await db.insert(apiKeyTable).values({
      id: keyId,
      userId: dbUser.id,
      keyHash: keyHash,
      label: label,
      createdAt: new Date(),
    });

    return new Response(
      JSON.stringify({
        id: keyId,
        label: label,
        apiKey: rawKey,
      }),
      {
        status: 200,
        headers: { "Content-Type": "application/json" },
      },
    );
  } catch (err: any) {
    console.error("API Key Generation Error:", err);
    return new Response(JSON.stringify({ error: err.message }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }
};
