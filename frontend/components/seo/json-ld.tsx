import { Organization, WebSite, WebPage } from "schema-dts";

interface JsonLdProps {
  type: "Organization" | "WebSite" | "WebPage";
  data: Organization | WebSite | WebPage;
}

export default function JsonLd({ type, data }: JsonLdProps) {
  return (
    <script
      type="application/ld+json"
      dangerouslySetInnerHTML={{
        __html: JSON.stringify({
          "@context": "https://schema.org",
          "@type": type,
          ...data,
        }),
      }}
    />
  );
}
