import type { APIRoute } from "astro";
import { eq } from "drizzle-orm";
import { db } from "../../../db";
import { apiKey } from "../../../db/schema";
import { bearerToken } from "../../../lib/desktop-auth";
import { hashSecret, privateHeaders } from "../../../lib/desktop-auth-protocol";

export const POST: APIRoute = async ({ request }) => {
  const token = bearerToken(request);
  if (!token)
    return Response.json(
      { error: "Unauthorized" },
      { status: 401, headers: privateHeaders },
    );
  await db.delete(apiKey).where(eq(apiKey.keyHash, hashSecret(token)));
  return new Response(null, { status: 204, headers: privateHeaders });
};
