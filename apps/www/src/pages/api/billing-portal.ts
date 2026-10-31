import type { APIRoute } from "astro";
import { stripe } from "../../lib/stripe";
import { db } from "../../db";
import { user as userTable } from "../../db/schema";
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

    const [dbUser] = await db
      .select()
      .from(userTable)
      .where(eq(userTable.id, sessionUser.id));

    if (!dbUser?.stripeCustomerId) {
      return new Response(
        JSON.stringify({ error: "No active Stripe customer profile found." }),
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
  } catch (err: any) {
    console.error("Billing Portal Error:", err);
    return new Response(JSON.stringify({ error: err.message }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }
};
