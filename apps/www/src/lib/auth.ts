import { getSecret } from "astro:env/server";
import { betterAuth } from "better-auth/minimal";
import { drizzleAdapter } from "@better-auth/drizzle-adapter";
import { db } from "../db";
import * as schema from "../db/schema";

const developmentOrigins: string[] = import.meta.env.DEV
  ? (getSecret("AUDEXIS_DEV_AUTH_ORIGINS") ?? "")
      .split(",")
      .map((value: string) => value.trim())
      .filter(Boolean)
      .map((value: string) => new URL(value).origin)
  : [];
const configuredOrigin =
  getSecret("BETTER_AUTH_URL")?.trim() ||
  (import.meta.env.DEV ? "http://localhost:4321" : "https://www.audexis.app");
const allowedOrigins = [
  ...new Set([
    ...(configuredOrigin ? [new URL(configuredOrigin).origin] : []),
    ...developmentOrigins,
  ]),
];

export const auth = betterAuth({
  secret: getSecret("BETTER_AUTH_SECRET"),
  baseURL: import.meta.env.DEV
    ? {
        allowedHosts: [...allowedOrigins.map((origin) => new URL(origin).host)],
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
      clientId: getSecret("GITHUB_CLIENT_ID")!,
      clientSecret: getSecret("GITHUB_CLIENT_SECRET")!,
    },
    google: {
      clientId: getSecret("GOOGLE_CLIENT_ID")!,
      clientSecret: getSecret("GOOGLE_CLIENT_SECRET")!,
    },
  },
});
