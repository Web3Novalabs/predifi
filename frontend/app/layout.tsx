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
  metadataBase: new URL(""),
  title: {
    default: "PrediFI - Onchain Prediction Protocol",
    template: "%s | PrediFI",
  },
  description:
    "Prediction Protocol built on starknet, predict various outcomes across various fields",
  keywords: [
    "prediction protocol",
    "starknet",
    "blockchain",
    "predictions",
    "web3",
    "defi",
  ],
  authors: [{ name: "PrediFI Team" }],
  creator: "PrediFI",
  publisher: "PrediFI",
  formatDetection: {
    email: false,
    address: false,
    telephone: false,
  },
  openGraph: {
    type: "website",
    locale: "en_US",
    url: "",
    siteName: "PrediFI",
    title: "PrediFI - Onchain Prediction Protocol",
    description:
      "Prediction Protocol built on starknet, predict various outcomes across various fields",
    images: [
      {
        url: "/code.png",
        width: 1200,
        height: 630,
        alt: "PrediFI - Onchain Prediction Protocol",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "PrediFI - Onchain Prediction Protocol",
    description:
      "Prediction Protocol built on starknet, predict various outcomes across various fields",
    images: ["/code.png"], // Same image as OpenGraph
    creator: "@predifi_",
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-video-preview": -1,
      "max-image-preview": "large",
      "max-snippet": -1,
    },
  },
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
