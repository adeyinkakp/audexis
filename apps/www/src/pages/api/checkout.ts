import type { APIRoute } from "astro";
import type Stripe from "stripe";
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

    if (!dbUser) {
      return new Response(JSON.stringify({ error: "User not found" }), {
        status: 404,
        headers: { "Content-Type": "application/json" },
      });
    }

    const isAlreadySubscribed =
      dbUser.subscriptionStatus === "active" ||
      dbUser.subscriptionStatus === "trialing";

    if (isAlreadySubscribed) {
      return new Response(
        JSON.stringify({
          error: "You already have an active subscription.",
          code: "ALREADY_SUBSCRIBED",
        }),
        {
          status: 409,
          headers: { "Content-Type": "application/json" },
        },
      );
    }

    const origin = new URL(request.url).origin;

    const sessionParams: Stripe.Checkout.SessionCreateParams = {
      mode: "subscription",
      client_reference_id: dbUser.id,
      line_items: [
        {
          price: import.meta.env.STRIPE_AUDEXIS_PRICE_ID,
          quantity: 1,
        },
      ],
      success_url: `${origin}/dashboard?payment=success`,
      cancel_url: `${origin}/dashboard?payment=cancelled`,
    };

    if (dbUser.stripeCustomerId) {
      sessionParams.customer = dbUser.stripeCustomerId;
    } else {
      sessionParams.customer_email = dbUser.email;
    }

    const session = await stripe.checkout.sessions.create(sessionParams);

    return new Response(JSON.stringify({ url: session.url }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  } catch (err: any) {
    console.error("Stripe Checkout Error:", err);
    return new Response(JSON.stringify({ error: err.message }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }
};
