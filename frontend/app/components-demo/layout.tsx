import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = { title: "Components Demo" };

export default function ComponentsDemoLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
