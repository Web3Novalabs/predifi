import type { Metadata } from "next";
import { ComingSoon } from "@/components/ui";

export const metadata: Metadata = {
  title: "Validator — Coming Soon",
  robots: { index: false, follow: false },
};

export default function Page() {
  return (
    <ComingSoon
      title="Validator"
      description="Validators will review prediction outcomes, vote on disputed results, and earn rewards for keeping markets honest. This feature isn't available yet — check back soon."
    />
  );
}
