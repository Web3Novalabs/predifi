import type { Metadata } from "next";
import { notFound } from "next/navigation";
import type { ReactNode } from "react";
import { isComponentsDemoEnabled } from "@/lib/componentsDemo";

export const metadata: Metadata = {
  title: "Components Demo",
  robots: { index: false, follow: false },
};

export default function ComponentsDemoLayout({ children }: { children: ReactNode }) {
  if (!isComponentsDemoEnabled()) notFound();
  return <>{children}</>;
}
