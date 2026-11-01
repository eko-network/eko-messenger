# PLAN: Add Messages to `eko`

This document plans the work needed to bring `eko-messenger` into the `../eko`
Flutter + Supabase project as the messaging backend.

The driving principles:

- Supabase is the source of truth for **user identity** (`auth.users`) and for the **device list** (`messenger.devices`).
- The Rust server is the source of truth for the **messaging protocol**: outbox validation, fanout, federation, online delivery (WebSocket), and push notifications (VAPID/web-push).
- A Supabase **custom-access-token auth hook** injects the current `did` into the JWT so the Rust middleware can read it without minting its own tokens.
- The current `auth-firebase` / `auth-oidc` feature flags get removed; the Rust binary only ever speaks Supabase auth.
- The Rust binary depends on a Rust library. The Rust binary serves as an ecample on how to integrate the eko message protocol into an existing project.

---

## 1. Phase 1 — Move `devices` to Supabase + `did` auth hook

After this phase, the Flutter app can register a messenger device, the next-refreshed JWT carries `did`, and the Rust server can verify it.

