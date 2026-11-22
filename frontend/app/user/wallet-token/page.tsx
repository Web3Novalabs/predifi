import type { Metadata } from "next";
import { ComingSoon } from "@/components/ui";

export const metadata: Metadata = {
  title: "Wallet & Tokens — Coming Soon",
  robots: { index: false, follow: false },
};

export default function Page() {
  return (
    <ComingSoon
      title="Wallet & Tokens"
      description="A dedicated place to view your XLM holdings, stake tokens, and track transaction history is on the way. For now, manage your wallet from the dashboard."
    />
  );
}
