import type { MetadataRoute } from "next";
import { SITE_URL as BASE_URL } from "@/lib/site";

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
