import type { APIRoute } from "astro";
import { bearerToken, desktopCredential } from "../../../lib/desktop-auth";
import {
  hasCloudSubscription,
  privateHeaders,
} from "../../../lib/desktop-auth-protocol";

export const GET: APIRoute = async ({ request }) => {
  const token = bearerToken(request);
  const credential = token ? await desktopCredential(token) : null;
  if (!credential)
    return Response.json(
      { error: "Unauthorized" },
      { status: 401, headers: privateHeaders },
    );
  if (!hasCloudSubscription(credential.subscriber))
    return Response.json(
      { error: "subscription_required" },
      { status: 403, headers: privateHeaders },
    );
  return Response.json({ authenticated: true }, { headers: privateHeaders });
};
