import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "CheapOracles.wtf - A price feed for every token",
  description:
    "ZK-verified price feeds from 10 DEX standards across every EVM chain. $0.01 per update. No subscriptions. No vendor contracts. No permission needed.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        <link
          href="https://fonts.googleapis.com/css2?family=Permanent+Marker&family=VT323&family=Rubik:wght@400;500;600;700;800;900&family=JetBrains+Mono:wght@400;500;600;700;800&display=swap"
          rel="stylesheet"
        />
      </head>
      <body className="antialiased">{children}</body>
    </html>
  );
}
