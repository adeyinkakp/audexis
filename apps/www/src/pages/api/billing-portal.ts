import type { APIRoute } from "astro";
import { stripe } from "../../lib/stripe";
import {
  billingErrorResponse,
  refreshSubscription,
} from "../../lib/subscriptions";

export const POST: APIRoute = async ({ request, locals }) => {
  try {
    const sessionUser = locals.user;
    if (!sessionUser) {
      return new Response(JSON.stringify({ error: "Unauthorized" }), {
        status: 401,
        headers: { "Content-Type": "application/json" },
      });
    }

    const dbUser = await refreshSubscription(sessionUser.id);

    if (!dbUser?.stripeCustomerId) {
      return new Response(
        JSON.stringify({
          error:
            "No billing profile is available. Return to the dashboard to start a new subscription.",
        }),
        { status: 400, headers: { "Content-Type": "application/json" } },
      );
    }

    const portalSession = await stripe.billingPortal.sessions.create({
      customer: dbUser.stripeCustomerId,
      return_url: `${new URL(request.url).origin}/dashboard`,
    });

    return new Response(JSON.stringify({ url: portalSession.url }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  } catch (err) {
    console.error("Billing Portal Error:", err);
    return billingErrorResponse(err);
  }
};
