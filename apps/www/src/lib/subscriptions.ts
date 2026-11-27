import Stripe from "stripe";
import { eq } from "drizzle-orm";
import { db } from "../db";
import { user } from "../db/schema";
import { stripe } from "./stripe";

const requestOptions = { timeout: 10_000, maxNetworkRetries: 1 };

export class BillingError extends Error {
  constructor(
    public code: string,
    message: string,
    public status = 409,
  ) {
    super(message);
  }
}

const missingCustomer = () =>
  new BillingError(
    "BILLING_CUSTOMER_UNAVAILABLE",
    "Your billing profile is no longer available. Refresh the page to start a new subscription.",
  );

export async function checkBillingCustomer(customerId: string) {
  try {
    const customer = await stripe.customers.retrieve(
      customerId,
      {},
      requestOptions,
    );
    if (customer.deleted) throw missingCustomer();
    return customer;
  } catch (error) {
    if (
      error instanceof Stripe.errors.StripeInvalidRequestError &&
      error.code === "resource_missing"
    ) {
      throw missingCustomer();
    }
    throw error;
  }
}

async function retrieveSubscription(subscriptionId: string) {
  try {
    return await stripe.subscriptions.retrieve(
      subscriptionId,
      {},
      requestOptions,
    );
  } catch (error) {
    if (
      error instanceof Stripe.errors.StripeInvalidRequestError &&
      error.code === "resource_missing"
    ) {
      throw new BillingError(
        "BILLING_SUBSCRIPTION_UNAVAILABLE",
        "Your previous subscription is no longer available. Refresh the page to start a new subscription.",
      );
    }
    throw error;
  }
}

export function billingErrorMessage(error: unknown) {
  return error instanceof BillingError
    ? error.message
    : "Billing is temporarily unavailable. Please try again shortly.";
}

export function billingErrorResponse(error: unknown) {
  if (
    error instanceof Stripe.errors.StripeInvalidRequestError &&
    error.code === "resource_missing" &&
    error.param === "customer"
  ) {
    error = missingCustomer();
  }
  return Response.json(
    {
      error: billingErrorMessage(error),
      code: error instanceof BillingError ? error.code : "BILLING_UNAVAILABLE",
    },
    {
      status: error instanceof BillingError ? error.status : 503,
      headers: { "Cache-Control": "no-store" },
    },
  );
}

export async function refreshSubscription(
  userId: string,
  checkoutSubscriptionId?: string,
) {
  return db.transaction(async (tx) => {
    let [subscriber] = await tx
      .select()
      .from(user)
      .where(eq(user.id, userId))
      .for("update");
    if (!subscriber) return undefined;
    const clearSubscription = async (clearCustomer: boolean) => {
      const [updated] = await tx
        .update(user)
        .set({
          ...(clearCustomer ? { stripeCustomerId: null } : {}),
          stripeSubscriptionId: null,
          subscriptionStatus: null,
          subscriptionTier: null,
          currentPeriodEnd: null,
        })
        .where(eq(user.id, userId))
        .returning();
      return updated;
    };
    if (subscriber.stripeCustomerId) {
      try {
        await checkBillingCustomer(subscriber.stripeCustomerId);
      } catch (error) {
        if (
          !(error instanceof BillingError) ||
          error.code !== "BILLING_CUSTOMER_UNAVAILABLE"
        )
          throw error;
        subscriber = await clearSubscription(true);
        if (!checkoutSubscriptionId) return subscriber;
      }
    }
    const subscriptionId =
      checkoutSubscriptionId ?? subscriber.stripeSubscriptionId;
    if (!subscriptionId) return subscriber;

    let subscription: Stripe.Subscription;
    try {
      subscription = await retrieveSubscription(subscriptionId);
    } catch (error) {
      if (
        !(error instanceof BillingError) ||
        error.code !== "BILLING_SUBSCRIPTION_UNAVAILABLE"
      )
        throw error;
      if (
        checkoutSubscriptionId &&
        checkoutSubscriptionId !== subscriber.stripeSubscriptionId
      )
        return subscriber;
      return clearSubscription(false);
    }
    const customerId =
      typeof subscription.customer === "string"
        ? subscription.customer
        : subscription.customer.id;
    if (
      subscriber.stripeCustomerId &&
      subscriber.stripeCustomerId !== customerId
    ) {
      throw new BillingError(
        "BILLING_ACCOUNT_MISMATCH",
        "Your subscription does not match your saved billing profile. Please contact support.",
      );
    }
    if (!subscriber.stripeCustomerId) {
      try {
        await checkBillingCustomer(customerId);
      } catch (error) {
        if (
          !(error instanceof BillingError) ||
          error.code !== "BILLING_CUSTOMER_UNAVAILABLE"
        )
          throw error;
        return clearSubscription(true);
      }
    }
    if (
      checkoutSubscriptionId &&
      subscriber.stripeSubscriptionId &&
      subscriber.stripeSubscriptionId !== subscription.id
    ) {
      try {
        const current = await retrieveSubscription(
          subscriber.stripeSubscriptionId,
        );
        if (current.created >= subscription.created) return subscriber;
      } catch (error) {
        if (
          !(error instanceof BillingError) ||
          error.code !== "BILLING_SUBSCRIPTION_UNAVAILABLE"
        )
          throw error;
      }
    }
    const periodEnds = subscription.items.data
      .map((item) => item.current_period_end)
      .filter((end) => Number.isFinite(end) && end > 0);
    const [updated] = await tx
      .update(user)
      .set({
        stripeCustomerId: customerId,
        stripeSubscriptionId: subscription.id,
        subscriptionStatus: subscription.status,
        subscriptionTier: ["active", "trialing"].includes(subscription.status)
          ? "standard"
          : null,
        currentPeriodEnd: periodEnds.length
          ? new Date(Math.min(...periodEnds) * 1000)
          : null,
      })
      .where(eq(user.id, subscriber.id))
      .returning();
    return updated;
  });
}
