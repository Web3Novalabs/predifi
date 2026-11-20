import type { MetadataRoute } from "next";
import { SITE_URL as BASE_URL } from "@/lib/site";

// Public, indexable routes only. Unfinished feature pages under
// /user/validator, /user/wallet-token and /user/live-market are intentionally
// excluded — they are marked `noindex` and render a "Coming soon" state.
const STATIC_ROUTES = [
  "/",
  "/about",
  "/benefits",
  "/community",
  "/faq",
  "/features",
  "/waitlist",
  "/search",
] as const;

export default function sitemap(): MetadataRoute.Sitemap {
  return STATIC_ROUTES.map((route) => ({
    url: `${BASE_URL}${route}`,
    lastModified: new Date(),
    changeFrequency: "weekly",
    priority: route === "/" ? 1 : 0.8,
  }));
}
