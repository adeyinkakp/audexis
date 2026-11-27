import { getSecret } from "astro:env/server";
import type { APIRoute } from "astro";
import { stripe } from "../../../lib/stripe";
import { db } from "../../../db";
import { user } from "../../../db/schema";
import { eq } from "drizzle-orm";
import type Stripe from "stripe";
import { refreshSubscription } from "../../../lib/subscriptions";

export const POST: APIRoute = async ({ request }) => {
  const signature = request.headers.get("stripe-signature");
  const signingSecret = getSecret("STRIPE_WEBHOOK_SECRET");

  if (!signature || !signingSecret) {
    return new Response("Missing signature or secret", { status: 400 });
  }

  const rawBody = await request.text();
  let event: Stripe.Event;

  try {
    event = stripe.webhooks.constructEvent(rawBody, signature, signingSecret);
  } catch (err: any) {
    console.error(`Webhook signature verification failed: ${err.message}`);
    return new Response(`Webhook Error: ${err.message}`, { status: 400 });
  }

  try {
    switch (event.type) {
      case "checkout.session.completed": {
        const session = event.data.object as Stripe.Checkout.Session;
        const userId = session.client_reference_id;

        if (!userId) break;

        if (session.mode !== "subscription" || !session.subscription) break;
        const subscriptionId =
          typeof session.subscription === "string"
            ? session.subscription
            : session.subscription.id;
        if (!(await refreshSubscription(userId, subscriptionId))) {
          throw new Error("Checkout account was not found");
        }

        break;
      }

      case "customer.subscription.created":
      case "customer.subscription.resumed":
      case "customer.subscription.paused":
      case "customer.subscription.updated":
      case "customer.subscription.deleted": {
        const subscription = event.data.object as Stripe.Subscription;

        const [dbUser] = await db
          .select()
          .from(user)
          .where(eq(user.stripeSubscriptionId, subscription.id));
        if (dbUser) {
          await refreshSubscription(dbUser.id);
        } else if (subscription.metadata.userId) {
          const [account] = await db
            .select()
            .from(user)
            .where(eq(user.id, subscription.metadata.userId));
          if (account && !account.stripeSubscriptionId) {
            await refreshSubscription(account.id, subscription.id);
          }
        }

        break;
      }
    }
  } catch (dbError) {
    console.error("Database update failed during webhook:", dbError);
    return new Response("Database error", { status: 500 });
  }

  return new Response(JSON.stringify({ received: true }), { status: 200 });
};
