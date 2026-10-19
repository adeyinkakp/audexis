import { betterAuth } from "better-auth/minimal";
import { drizzleAdapter } from "@better-auth/drizzle-adapter";
import { db } from "../db";
import * as schema from "../db/schema";

export const auth = betterAuth({
  database: drizzleAdapter(db, {
    provider: "pg",
    schema: {
      ...schema,
    },
  }),
  user: {
    additionalFields: {
      stripeCustomerId: {
        type: "string",
        required: false,
      },
      stripeSubscriptionId: {
        type: "string",
        required: false,
      },
      subscriptionStatus: {
        type: "string",
        required: false,
      },
      subscriptionTier: {
        type: "string",
        required: false,
      },
      currentPeriodEnd: {
        type: "date",
        required: false,
      },
    },
  },
  emailAndPassword: {
    enabled: false,
  },
  account: {
    accountLinking: {
      disableImplicitLinking: true,
    },
  },
  socialProviders: {
    github: {
      clientId: import.meta.env.GITHUB_CLIENT_ID!,
      clientSecret: import.meta.env.GITHUB_CLIENT_SECRET!,
    },
    google: {
      clientId: import.meta.env.GOOGLE_CLIENT_ID!,
      clientSecret: import.meta.env.GOOGLE_CLIENT_SECRET!,
    },
  },
});
