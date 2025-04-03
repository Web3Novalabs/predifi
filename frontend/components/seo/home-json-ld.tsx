import JsonLd from "./json-ld";
import { WebSite } from "schema-dts";

export default function HomeJsonLd() {
  const websiteData: WebSite = {
    name: "PrediFI",
    description:
      "Transform Predictions Into Profits! Create and participate in decentralized prediction markets across sports, finance, and pop culture.",
    url: "",
    potentialAction: {
      "@type": "SearchAction",
      target: {
        "@type": "EntryPoint",
        urlTemplate: "",
      },
      "query-input": "required name=search_term_string",
    },
  };

  return <JsonLd type="WebSite" data={websiteData} />;
}
