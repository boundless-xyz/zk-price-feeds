import type { Metadata } from "next";
import { Space_Grotesk, DM_Sans, JetBrains_Mono } from "next/font/google";
import "./globals.css";

const spaceGrotesk = Space_Grotesk({
  subsets: ["latin"],
  variable: "--font-space-grotesk",
  display: "swap",
});

const dmSans = DM_Sans({
  subsets: ["latin"],
  variable: "--font-dm-sans",
  display: "swap",
});

const jetBrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains-mono",
  display: "swap",
});

export const metadata: Metadata = {
  title: "AnyFeed — A price feed for every token",
  description:
    "ZK-verified DEX price feeds for any on-chain pair. No subscriptions. No vendor contracts. No permission needed. $0.01 per update on L2.",
  icons: {
    icon: "/favicon.svg",
  },
  openGraph: {
    title: "AnyFeed — A price feed for every token",
    description:
      "ZK-verified DEX price feeds for any on-chain pair. $0.01 per update. Chainlink-compatible interface.",
    type: "website",
    siteName: "AnyFeed",
  },
  twitter: {
    card: "summary_large_image",
    title: "AnyFeed — A price feed for every token",
    description:
      "ZK-verified DEX price feeds for any on-chain pair. $0.01 per update. Chainlink-compatible.",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        <link rel="preconnect" href="https://api.dexscreener.com" />
      </head>
      <body
        className={`${spaceGrotesk.variable} ${dmSans.variable} ${jetBrainsMono.variable} antialiased`}
      >
        {children}
      </body>
    </html>
  );
}
