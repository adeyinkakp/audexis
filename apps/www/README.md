# Audexis Website

The Audexis website, documentation, and account dashboard. Built with Astro
and Tailwind CSS.

## Development

Use Node.js 22.12 or newer and pnpm. From this directory:

```sh
pnpm install
pnpm dev
```

Build the site with:

```sh
pnpm build
```

## Configuration

Local environment values belong in `.env` in this directory. The account and
billing routes use the following settings:

| Variable                                   | Purpose                                             |
| ------------------------------------------ | --------------------------------------------------- |
| `DATABASE_URL`                             | PostgreSQL connection for accounts and billing data |
| `BETTER_AUTH_URL`                          | Website authentication origin                       |
| `BETTER_AUTH_SECRET`                       | Better Auth signing secret                          |
| `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET` | GitHub sign-in credentials                          |
| `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` | Google sign-in credentials                          |
| `AUDEXIS_DEV_AUTH_ORIGINS`                 | Comma-separated additional development origins      |
| `STRIPE_SECRET_KEY`                        | Stripe server API key                               |
| `STRIPE_WEBHOOK_SECRET`                    | Stripe webhook verification secret                  |
| `STRIPE_AUDEXIS_PRICE_ID`                  | Subscription price used by checkout                 |

The database schema is in `src/db/schema.ts`; Drizzle tooling is configured
in `drizzle.config.ts`.

## Code map

| Path                 | Purpose                                              |
| -------------------- | ---------------------------------------------------- |
| `src/pages/`         | Website pages and API routes                         |
| `src/pages/docs/`    | Markdown product documentation                       |
| `src/lib/`           | Authentication, desktop sign-in, and billing helpers |
| `src/db/`            | Database connection and schema                       |
| `src/assets/styles/` | Site styles                                          |
| `public/`            | More static assets                                   |
