import { getSecret } from "astro:env/server";
import Stripe from "stripe";

export const stripe = new Stripe(getSecret("STRIPE_SECRET_KEY")!, {
  apiVersion: "2026-09-30.endive",
  typescript: true,
});
