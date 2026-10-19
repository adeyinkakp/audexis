import type { APIRoute } from "astro";
import { stripe } from "../../../lib/stripe";
import { db } from "../../../db";
import { user } from "../../../db/schema";
import { eq } from "drizzle-orm";
import type Stripe from "stripe";

export const POST: APIRoute = async ({ request }) => {
  const signature = request.headers.get("stripe-signature");
  const signingSecret = import.meta.env.STRIPE_WEBHOOK_SECRET;

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

        const subscription = await stripe.subscriptions.retrieve(
          session.subscription as string,
        );

        await db
          .update(user)
          .set({
            stripeCustomerId: session.customer as string,
            stripeSubscriptionId: subscription.id,
            subscriptionStatus: subscription.status,
            subscriptionTier: "standard",
            currentPeriodEnd: new Date(
              subscription.items.data[0].current_period_end * 1000,
            ),
          })
          .where(eq(user.id, userId));

        break;
      }

      case "customer.subscription.updated":
      case "customer.subscription.deleted": {
        const subscription = event.data.object as Stripe.Subscription;

        const [dbUser] = await db
          .select()
          .from(user)
          .where(eq(user.stripeCustomerId, subscription.customer as string));

        if (!dbUser) break;

        await db
          .update(user)
          .set({
            subscriptionStatus: subscription.status,
            currentPeriodEnd: new Date(
              subscription.items.data[0].current_period_end * 1000,
            ),
          })
          .where(eq(user.id, dbUser.id));

        break;
      }
    }
  } catch (dbError) {
    console.error("Database update failed during webhook:", dbError);
    return new Response("Database error", { status: 500 });
  }

  return new Response(JSON.stringify({ received: true }), { status: 200 });
};
