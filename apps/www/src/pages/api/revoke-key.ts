import type { APIRoute } from "astro";
import { db } from "../../db";
import { apiKey as apiKeyTable } from "../../db/schema";
import { and, eq } from "drizzle-orm";
import {
  privateHeaders,
  sameOriginPost,
} from "../../lib/desktop-auth-protocol";

export const POST: APIRoute = async ({ request, locals }) => {
  if (!sameOriginPost(request))
    return Response.json(
      { error: "Invalid request origin" },
      { status: 403, headers: privateHeaders },
    );
  try {
    const sessionUser = locals.user;
    if (!sessionUser) {
      return new Response(JSON.stringify({ error: "Unauthorized" }), {
        status: 401,
        headers: { "Content-Type": "application/json" },
      });
    }

    const { keyId } = await request.json();
    if (!keyId) {
      return new Response(JSON.stringify({ error: "Missing keyId" }), {
        status: 400,
        headers: { "Content-Type": "application/json" },
      });
    }

    await db
      .delete(apiKeyTable)
      .where(
        and(eq(apiKeyTable.id, keyId), eq(apiKeyTable.userId, sessionUser.id)),
      );

    return new Response(JSON.stringify({ success: true }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  } catch (err: any) {
    console.error("Revoke Key Error:", err);
    return new Response(JSON.stringify({ error: "Could not revoke API key" }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }
};
