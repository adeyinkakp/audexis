/// <reference path="../.astro/types.d.ts" />

type User = typeof import("./lib/auth").auth.$Infer.Session.user;
type Session = typeof import("./lib/auth").auth.$Infer.Session.session;

declare namespace App {
  interface Locals {
    user: User | null;
    session: Session | null;
  }
}
