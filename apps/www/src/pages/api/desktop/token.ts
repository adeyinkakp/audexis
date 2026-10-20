import type { APIRoute } from "astro";
import { redeemDesktopCode } from "../../../lib/desktop-auth";
import { privateHeaders } from "../../../lib/desktop-auth-protocol";

export const POST: APIRoute = async ({ request }) => {
  if (!request.headers.get("content-type")?.startsWith("application/json")) {
    return Response.json(
      { error: "invalid_request" },
      { status: 415, headers: privateHeaders },
    );
  }
  try {
    const body = await request.json();
    if (body?.grant_type !== "authorization_code") {
      return Response.json(
        { error: "invalid_request" },
        { status: 400, headers: privateHeaders },
      );
    }
    const token = await redeemDesktopCode(
      body.code,
      body.code_verifier,
      body.redirect_uri,
    );
    if (!token)
      return Response.json(
        { error: "invalid_grant" },
        { status: 400, headers: privateHeaders },
      );
    return Response.json(
      { access_token: token, token_type: "Bearer" },
      { headers: privateHeaders },
    );
  } catch {
    return Response.json(
      { error: "Could not complete sign-in. Please try again." },
      { status: 400, headers: privateHeaders },
    );
  }
};
