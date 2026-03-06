"use client";

import { useState } from "react";

const tabs = [
  {
    name: "Chainlink",
    color: "text-chainlink-blue",
    borderColor: "border-chainlink-blue",
    bgActive: "bg-chainlink-blue/10",
    before: {
      iface: "AggregatorV3Interface",
      constructor: "AggregatorV3Interface",
      address: "0x5f4eC3Df...5b8419",
      comment: "// Chainlink ETH/USD",
    },
    after: {
      iface: "AggregatorV3Interface",
      constructor: "AggregatorV3Interface",
      address: "0x...",
      comment: "// ZK Oracle -- proven by math",
    },
    usage: `// Your code stays exactly the same
(, int256 price, , uint256 updatedAt, ) = feed.latestRoundData();`,
  },
  {
    name: "Pyth",
    color: "text-pyth-purple",
    borderColor: "border-pyth-purple",
    bgActive: "bg-pyth-purple/10",
    before: {
      iface: "IPyth",
      constructor: "IPyth",
      address: "0xff1a0f4744e8582DF...60aE",
      comment: "// Pyth mainnet",
    },
    after: {
      iface: "IPyth",
      constructor: "IPyth",
      address: "0x...",
      comment: "// ZK Oracle -- proven by math",
    },
    usage: `// Your code stays exactly the same
PythStructs.Price memory p = pyth.getPriceUnsafe(priceId);
int64 price = p.price;`,
  },
  {
    name: "RedStone",
    color: "text-redstone-red",
    borderColor: "border-redstone-red",
    bgActive: "bg-redstone-red/10",
    before: {
      iface: "IRedstoneAdapter",
      constructor: "IRedstoneAdapter",
      address: "0x5300...dB16",
      comment: "// RedStone adapter",
    },
    after: {
      iface: "IRedstoneAdapter",
      constructor: "IRedstoneAdapter",
      address: "0x...",
      comment: "// ZK Oracle -- proven by math",
    },
    usage: `// Your code stays exactly the same
uint256 price = adapter.getValueForDataFeed("ETH");`,
  },
];

export default function ZeroMigration() {
  const [active, setActive] = useState(0);
  const tab = tabs[active];

  return (
    <section className="py-28 px-6 relative overflow-hidden">
      <div className="absolute top-0 left-0 w-[400px] h-[400px] bg-neon-purple/5 rounded-full blur-[150px]" />

      <div className="max-w-4xl mx-auto relative z-10">
        <div className="text-center mb-14">
          <h2 className="text-4xl sm:text-6xl font-display tracking-wide">
            <span className="text-text-primary block">Already using an oracle?</span>
            <span className="text-neon-cyan block text-3xl sm:text-5xl mt-2">Change one address.</span>
          </h2>
        </div>

        {/* Code block with tabs */}
        <div className="glass-strong rounded-2xl overflow-hidden glow-purple">
          {/* Tab bar + window chrome */}
          <div className="flex items-center gap-3 px-6 py-3.5 border-b border-white/10">
            <div className="flex gap-2 mr-3" aria-hidden="true">
              <div className="w-3.5 h-3.5 rounded-full bg-neon-pink/70" />
              <div className="w-3.5 h-3.5 rounded-full bg-neon-yellow/70" />
              <div className="w-3.5 h-3.5 rounded-full bg-neon-green/70" />
            </div>
            <div className="flex gap-1">
              {tabs.map((t, i) => (
                <button
                  key={t.name}
                  onClick={() => setActive(i)}
                  className={`px-4 py-1.5 rounded-lg text-sm font-mono font-bold transition-all duration-300 ${
                    active === i
                      ? `${t.color} ${t.bgActive} ${t.borderColor} border`
                      : "text-text-muted hover:text-text-secondary border border-transparent"
                  }`}
                >
                  {t.name}
                </button>
              ))}
            </div>
            <span className="text-text-muted text-sm font-mono tracking-wider ml-auto hidden sm:block">YourProtocol.sol</span>
          </div>

          <pre className="p-6 sm:p-8 overflow-x-auto text-sm sm:text-base leading-relaxed font-mono">
            <code>
              <span className="text-text-muted">{"// Before"}</span>
              {"\n"}
              <span className="text-neon-cyan">{tab.before.iface}</span>
              <span className="text-text-primary">{" feed = "}</span>
              <span className="text-neon-cyan">{tab.before.constructor}</span>
              <span className="text-text-primary">{"("}</span>
              {"\n"}
              <span className="text-neon-orange">{"    "}{tab.before.address}</span>
              <span className="text-text-muted">{"  "}{tab.before.comment}</span>
              {"\n"}
              <span className="text-text-primary">{");"}</span>
              {"\n\n"}
              <span className="text-neon-green font-bold">{"// After"}</span>
              {"\n"}
              <span className="text-neon-cyan">{tab.after.iface}</span>
              <span className="text-text-primary">{" feed = "}</span>
              <span className="text-neon-cyan">{tab.after.constructor}</span>
              <span className="text-text-primary">{"("}</span>
              {"\n"}
              <span className="text-neon-green font-bold">{"    "}{tab.after.address}</span>
              <span className="text-neon-green">{"  "}{tab.after.comment}</span>
              {"\n"}
              <span className="text-text-primary">{");"}</span>
              {"\n\n"}
              {tab.usage.split("\n").map((line, i) => {
                const isComment = line.startsWith("//");
                return (
                  <span key={i}>
                    {isComment ? (
                      <span className="text-text-muted">{line}</span>
                    ) : (
                      <span className="text-text-primary">{line}</span>
                    )}
                    {i < tab.usage.split("\n").length - 1 && "\n"}
                  </span>
                );
              })}
            </code>
          </pre>
        </div>

        {/* Supported standards */}
        <div className="mt-12 grid sm:grid-cols-2 lg:grid-cols-4 gap-4 max-w-4xl mx-auto">
          {[
            { standard: "Chainlink", iface: "AggregatorV3Interface", color: "text-chainlink-blue", borderColor: "border-chainlink-blue/30" },
            { standard: "Pyth", iface: "IPyth", color: "text-pyth-purple", borderColor: "border-pyth-purple/30" },
            { standard: "Chronicle", iface: "IChronicle", color: "text-chronicle-teal", borderColor: "border-chronicle-teal/30" },
            { standard: "API3", iface: "IProxy", color: "text-neon-orange", borderColor: "border-neon-orange/30" },
          ].map((s) => (
            <div key={s.standard} className={`glass rounded-xl p-4 text-center border ${s.borderColor} hover:bg-white/[0.06] transition-all duration-300`}>
              <div className={`text-sm font-bold font-body ${s.color} mb-1`}>{s.standard}</div>
              <div className="text-xs font-mono text-text-muted">{s.iface}</div>
            </div>
          ))}
        </div>

        <p className="text-center text-text-secondary mt-8 max-w-2xl mx-auto font-body text-lg">
          We implement the standard interfaces from every major oracle provider.
          Your lending protocol, your DEX, your vault -- they all just work.
        </p>

        {/* Mid-page CTA */}
        <div className="mt-16 text-center glass-strong rounded-2xl p-8">
          <p className="text-xl font-body font-bold text-text-primary mb-4">
            Ready to try it? Deploy a feed for your token in 5 minutes.
          </p>
          <div className="flex items-center justify-center gap-4 flex-wrap">
            <a
              href="https://github.com/boundless-xyz/zk-oracle"
              target="_blank"
              rel="noopener noreferrer"
              className="px-6 py-3 rounded-xl font-bold text-bg transition-all duration-300 hover:-translate-y-1 glow-green"
              style={{ background: "linear-gradient(135deg, #39ff14, #00e5ff)" }}
            >
              Get Started on GitHub
            </a>
            <a
              href="https://docs.beboundless.xyz"
              target="_blank"
              rel="noopener noreferrer"
              className="glass px-6 py-3 rounded-xl text-text-primary font-bold hover:bg-white/10 transition-all duration-300 hover:-translate-y-1"
            >
              Read the Docs
            </a>
          </div>
        </div>
      </div>
    </section>
  );
}
