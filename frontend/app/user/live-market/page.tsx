import type { Metadata } from "next";
import { ComingSoon } from "@/components/ui";

export const metadata: Metadata = {
  title: "Live Market — Coming Soon",
  robots: { index: false, follow: false },
};

export default function Page() {
  return (
    <ComingSoon
      title="Live Market"
      description="A real-time view of live prediction pools with up-to-the-second odds is coming soon. In the meantime, browse open pools from the market."
    />
  );
}
