/**
 * Canonical public origin of this deployment, without a trailing slash.
 * Derived from NEXT_PUBLIC_SITE_URL so staging, preview and self-hosted
 * deployments advertise their own URLs, falling back to production.
 */
export const SITE_URL: string =
  process.env.NEXT_PUBLIC_SITE_URL?.replace(/\/$/, "") || "https://predifi.app";
