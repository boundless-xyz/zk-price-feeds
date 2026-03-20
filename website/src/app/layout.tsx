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
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify({
              "@context": "https://schema.org",
              "@type": "SoftwareApplication",
              "name": "AnyFeed",
              "description": "ZK-verified DEX price feeds for any on-chain token pair. 500,000+ pairs supported. $0.01 per update on L2. Chainlink-compatible interface.",
              "applicationCategory": "DeveloperApplication",
              "operatingSystem": "EVM-compatible blockchains",
              "url": "https://github.com/boundless-xyz/zk-price-feeds",
              "offers": {
                "@type": "Offer",
                "price": "0.01",
                "priceCurrency": "USD",
                "description": "Per price feed update on L2"
              },
              "author": {
                "@type": "Organization",
                "name": "Boundless Networks",
                "url": "https://boundless.xyz",
                "sameAs": [
                  "https://github.com/boundless-xyz",
                  "https://twitter.com/beboundless"
                ]
              }
            }),
          }}
        />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify({
              "@context": "https://schema.org",
              "@type": "FAQPage",
              "mainEntity": [
                {
                  "@type": "Question",
                  "name": "What is AnyFeed?",
                  "acceptedAnswer": {
                    "@type": "Answer",
                    "text": "AnyFeed provides ZK-verified DEX price feeds for any on-chain token pair. It supports 500,000+ trading pairs across 8 DEX types, costs $0.01 per update on L2, and offers a Chainlink-compatible interface requiring zero code changes."
                  }
                },
                {
                  "@type": "Question",
                  "name": "How is AnyFeed flash-loan resistant?",
                  "acceptedAnswer": {
                    "@type": "Answer",
                    "text": "AnyFeed proves prices across a contiguous range of 10+ blocks and takes the median. Flash loans execute within a single block and cannot influence the median across a range. The range is configurable for stronger guarantees."
                  }
                },
                {
                  "@type": "Question",
                  "name": "How much does AnyFeed cost?",
                  "acceptedAnswer": {
                    "@type": "Answer",
                    "text": "AnyFeed costs approximately $0.01 per price feed update on L2, broken down as $0.004 for ZK proving on the Boundless marketplace and $0.007 for on-chain gas. No subscriptions or vendor contracts required."
                  }
                },
                {
                  "@type": "Question",
                  "name": "Can I use AnyFeed with my existing Chainlink integration?",
                  "acceptedAnswer": {
                    "@type": "Answer",
                    "text": "Yes. AnyFeed implements the AggregatorV3Interface, so it is a drop-in replacement for Chainlink. Change one address in your contract and the rest of your code stays the same. Also compatible with Pyth, RedStone, and Chronicle interfaces."
                  }
                }
              ]
            }),
          }}
        />
      </head>
      <body
        className={`${spaceGrotesk.variable} ${dmSans.variable} ${jetBrainsMono.variable} antialiased`}
      >
        {children}
      </body>
    </html>
  );
}
