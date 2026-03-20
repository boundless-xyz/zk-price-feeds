"use client";

import { useState } from "react";
import Reveal from "./Reveal";

const tabs = [
  {
    name: "Chainlink",
    before: {
      iface: "AggregatorV3Interface",
      addr: "0x5f4eC3Df...5b8419",
      comment: "// Chainlink ETH/USD",
    },
    after: {
      addr: "0x...",
      comment: "// ZK price feed -- proven by math",
    },
    usage: `// Your code stays exactly the same
(, int256 price, , uint256 updatedAt, ) = feed.latestRoundData();`,
  },
  {
    name: "Pyth",
    before: {
      iface: "IPyth",
      addr: "0xff1a0f47...60aE",
      comment: "// Pyth mainnet",
    },
    after: {
      addr: "0x...",
      comment: "// ZK price feed -- proven by math",
    },
    usage: `// Your code stays exactly the same
PythStructs.Price memory p = pyth.getPriceUnsafe(priceId);
int64 price = p.price;`,
  },
  {
    name: "RedStone",
    before: {
      iface: "IRedstoneAdapter",
      addr: "0x5300...dB16",
      comment: "// RedStone adapter",
    },
    after: {
      addr: "0x...",
      comment: "// ZK price feed -- proven by math",
    },
    usage: `// Your code stays exactly the same
uint256 price = adapter.getValueForDataFeed("ETH");`,
  },
  {
    name: "Chronicle",
    before: {
      iface: "IChronicle",
      addr: "0xc8A1F9...1b2E",
      comment: "// Chronicle ETH/USD",
    },
    after: {
      addr: "0x...",
      comment: "// ZK price feed -- proven by math",
    },
    usage: `// Your code stays exactly the same
uint256 price = chronicle.read();`,
  },
];

export default function ZeroMigration() {
  const [active, setActive] = useState(0);
  const tab = tabs[active];

  return (
    <section className="py-20 sm:py-28 border-t border-border">
      <div className="max-w-5xl mx-auto px-5 sm:px-8">
        <Reveal>
          <div className="text-center mb-10 sm:mb-14">
            <h2 className="text-3xl sm:text-4xl font-display font-bold tracking-tight text-text-primary mb-2">
              Already using a price feed? Paying too much on a contract?
            </h2>
            <p className="text-xl sm:text-2xl font-display font-bold text-accent tracking-tight">
              Switch now. Change one address.
            </p>
          </div>
        </Reveal>

        {/* Code block — centered */}
        <Reveal>
          <div className="max-w-3xl mx-auto">
            {/* Tab row */}
            <div className="flex items-center justify-center gap-2 mb-4">
              {tabs.map((t, i) => (
                <button
                  key={t.name}
                  role="tab"
                  aria-selected={active === i}
                  onClick={() => setActive(i)}
                  className={`px-4 py-2 rounded-full text-sm font-body font-medium transition-colors ${
                    active === i
                      ? "bg-accent text-white"
                      : "bg-bg-surface text-text-secondary hover:text-text-primary"
                  }`}
                >
                  {t.name}
                </button>
              ))}
            </div>

            {/* Code block */}
            <div className="bg-code-bg rounded-xl overflow-hidden">
              <div className="flex items-center justify-between px-4 sm:px-5 py-3 border-b border-code-surface">
                <div className="flex gap-1.5" aria-hidden="true">
                  <div className="w-2.5 h-2.5 rounded-full bg-code-muted/40" />
                  <div className="w-2.5 h-2.5 rounded-full bg-code-muted/40" />
                  <div className="w-2.5 h-2.5 rounded-full bg-code-muted/40" />
                </div>
                <span className="text-code-muted text-xs font-mono">
                  YourProtocol.sol
                </span>
              </div>

              <pre className="p-4 sm:p-6 overflow-x-auto text-xs sm:text-sm leading-relaxed font-mono">
                <code>
                  <span className="text-code-muted">{"// Before"}</span>
                  {"\n"}
                  <span className="text-code-keyword">{tab.before.iface}</span>
                  <span className="text-code-text">{" feed = "}</span>
                  <span className="text-code-keyword">{tab.before.iface}</span>
                  <span className="text-code-text">{"("}</span>
                  {"\n"}
                  <span className="text-code-text/60">
                    {"    "}
                    {tab.before.addr}
                  </span>
                  <span className="text-code-muted">
                    {"  "}
                    {tab.before.comment}
                  </span>
                  {"\n"}
                  <span className="text-code-text">{");"}</span>
                  {"\n\n"}
                  <span className="text-code-accent font-bold">
                    {"// After"}
                  </span>
                  {"\n"}
                  <span className="text-code-keyword">{tab.before.iface}</span>
                  <span className="text-code-text">{" feed = "}</span>
                  <span className="text-code-keyword">{tab.before.iface}</span>
                  <span className="text-code-text">{"("}</span>
                  {"\n"}
                  <span className="text-code-accent">
                    {"    "}
                    {tab.after.addr}
                  </span>
                  <span className="text-code-accent">
                    {"  "}
                    {tab.after.comment}
                  </span>
                  {"\n"}
                  <span className="text-code-text">{");"}</span>
                  {"\n\n"}
                  {tab.usage.split("\n").map((line, i) => (
                    <span key={i}>
                      {line.startsWith("//") ? (
                        <span className="text-code-muted">{line}</span>
                      ) : (
                        <span className="text-code-text">{line}</span>
                      )}
                      {i < tab.usage.split("\n").length - 1 && "\n"}
                    </span>
                  ))}
                </code>
              </pre>
            </div>
          </div>
        </Reveal>

        {/* Supported standards */}
        <Reveal>
          <div className="mt-10 sm:mt-12 max-w-3xl mx-auto grid grid-cols-2 sm:grid-cols-4 gap-3">
            {[
              { standard: "Chainlink", iface: "AggregatorV3Interface" },
              { standard: "Pyth", iface: "IPyth" },
              { standard: "Chronicle", iface: "IChronicle + IToll" },
              { standard: "RedStone", iface: "IRedstoneAdapter" },
            ].map((s) => (
              <div
                key={s.standard}
                className="bg-bg-surface rounded-xl p-3 sm:p-4 hover:bg-bg-raised transition-colors"
              >
                <div className="text-sm font-medium font-body text-text-primary mb-0.5">
                  {s.standard}
                </div>
                <div className="text-xs font-mono text-text-muted">
                  {s.iface}
                </div>
              </div>
            ))}
          </div>
        </Reveal>

        <Reveal>
          <p className="text-text-secondary mt-6 max-w-3xl mx-auto text-center font-body text-sm leading-relaxed">
            We implement the standard interfaces from every major oracle
            provider. Your lending protocol, your DEX, your vault -- they all
            just work.
          </p>
        </Reveal>
      </div>
    </section>
  );
}
