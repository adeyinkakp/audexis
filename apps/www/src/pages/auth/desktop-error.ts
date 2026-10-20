import type { APIRoute } from "astro";
import {
  desktopErrorRedirect,
  privateHeaders,
} from "../../lib/desktop-auth-protocol";

export const GET: APIRoute = ({ url }) => {
  const target = desktopErrorRedirect(
    url.searchParams.get("callback"),
    url.origin,
    url.searchParams.get("error"),
  );
  return new Response(null, {
    status: 303,
    headers: {
      ...privateHeaders,
      Location: target ?? "/login?error=sign_in_failed",
    },
  });
};
