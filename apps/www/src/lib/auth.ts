import { betterAuth } from "better-auth/minimal";
import { drizzleAdapter } from "@better-auth/drizzle-adapter";
import { db } from "../db";
import * as schema from "../db/schema";

const developmentOrigins: string[] = import.meta.env.DEV
  ? (import.meta.env.AUDEXIS_DEV_AUTH_ORIGINS ?? "")
      .split(",")
      .map((value: string) => value.trim())
      .filter(Boolean)
      .map((value: string) => new URL(value).origin)
  : [];
const configuredOrigin = import.meta.env.BETTER_AUTH_URL;
const allowedOrigins = [
  ...new Set([
    ...(configuredOrigin ? [new URL(configuredOrigin).origin] : []),
    ...developmentOrigins,
  ]),
];

export const auth = betterAuth({
  baseURL: import.meta.env.DEV
    ? {
        allowedHosts: allowedOrigins.map((origin) => new URL(origin).host),
        protocol: "auto",
      }
    : configuredOrigin,
  trustedOrigins: allowedOrigins,
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
        input: false,
      },
      stripeSubscriptionId: {
        type: "string",
        required: false,
        input: false,
      },
      subscriptionStatus: {
        type: "string",
        required: false,
        input: false,
      },
      subscriptionTier: {
        type: "string",
        required: false,
        input: false,
      },
      currentPeriodEnd: {
        type: "date",
        required: false,
        input: false,
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
