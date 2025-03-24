import type { Metadata } from "next";
import { Jersey_10, Work_Sans } from "next/font/google";
import "./globals.css";
import Footer from "@/components/layout/footer";
import Nav from "@/components/layout/nav";
import StarknetProvider from "@/components/starknet-provider";
import FilterContextProvider from "@/context/filter-context-provider";
import AllFilterContextProvider from "@/context/all-contex-provider";
import Script from "next/script";
import { Toaster } from "@/components/ui/sonner";
import ThemeProvider from "@/components/layout/Themeprovider";

const Jersey10 = Jersey_10({
  subsets: ["latin"],
  weight: "400",
  variable: "--font-jersey-10",
});

const WorkSans = Work_Sans({
  variable: "--font-work-sans",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "PrediFI - Onchain Prediction Protocol",
  description:
    "Prediction Protocol built on starknet, predict various outcomes across various fields",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        {" "}
        <Script src="https://telegram.org/js/telegram-web-app.js"></Script>
      </head>
      <body
        className={`${Jersey10.variable} ${WorkSans.variable} antialiased text-[#FFFFFF] font-work bg-[#100e16]`}
      >
        <ThemeProvider>
          <StarknetProvider>
            <Nav />
            <AllFilterContextProvider>
              <FilterContextProvider>
                <section className="max-w-screen-[1500px] mx-auto min-h-screen pb-14">
                  {children}
                </section>
              </FilterContextProvider>
            </AllFilterContextProvider>
            <Footer />
            <Toaster
              toastOptions={{
                unstyled: true,
                classNames: {
                  error: "toaster toast-error",
                  success: "toaster toast-success",
                  warning: "toaster toast-warning",
                  info: "toaster toast-info",
                },
              }}
            />
          </StarknetProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
