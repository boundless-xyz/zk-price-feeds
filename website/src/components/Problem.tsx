const categories = [
  "Micro-cap DeFi tokens",
  "LP tokens & vault shares",
  "New launches (day-one feeds)",
  "Community tokens",
  "Cross-chain bridged assets",
  "RWA tokens without CEX listings",
];

const tokens = [
  { s: "ETH", lit: true }, { s: "BTC", lit: true }, { s: "LINK", lit: true },
  { s: "UNI", lit: true }, { s: "?", lit: false }, { s: "?", lit: false },
  { s: "AAVE", lit: true }, { s: "?", lit: false }, { s: "?", lit: false },
  { s: "?", lit: false }, { s: "MKR", lit: true }, { s: "?", lit: false },
  { s: "?", lit: false }, { s: "?", lit: false }, { s: "?", lit: false },
  { s: "CRV", lit: true }, { s: "?", lit: false }, { s: "?", lit: false },
  { s: "?", lit: false }, { s: "?", lit: false }, { s: "?", lit: false },
  { s: "?", lit: false }, { s: "SNX", lit: true }, { s: "?", lit: false },
  { s: "?", lit: false }, { s: "?", lit: false }, { s: "?", lit: false },
  { s: "?", lit: false }, { s: "?", lit: false }, { s: "?", lit: false },
  { s: "COMP", lit: true }, { s: "?", lit: false }, { s: "?", lit: false },
  { s: "?", lit: false }, { s: "?", lit: false }, { s: "?", lit: false },
];

const dexes = [
  { name: "Uniswap V2/V3", chains: "Ethereum, Base, Arbitrum, ..." },
  { name: "PancakeSwap", chains: "BSC, Ethereum, Base, ..." },
  { name: "Curve", chains: "Ethereum, Arbitrum, ..." },
  { name: "Balancer V2", chains: "Ethereum, Arbitrum, Polygon" },
  { name: "Aerodrome", chains: "Base" },
  { name: "Trader Joe", chains: "Avalanche, Arbitrum" },
  { name: "DODO", chains: "Multi-chain" },
  { name: "Camelot", chains: "Arbitrum" },
  { name: "SushiSwap", chains: "Multi-chain" },
  { name: "Any V2 fork", chains: "Any EVM chain" },
];

export default function Problem() {
  return (
    <section className="py-28 px-6 relative overflow-hidden">
      <div className="absolute top-0 right-0 w-[400px] h-[400px] bg-neon-purple/5 rounded-full blur-[150px]" />

      <div className="max-w-5xl mx-auto relative z-10">
        <div className="text-center mb-12">
          <h2 className="text-4xl sm:text-6xl font-display tracking-wide mb-6">
            <span className="text-text-primary block">Chainlink covers <span className="text-neon-cyan text-glow-cyan">~350 tokens</span>.</span>
            <span className="text-text-primary block mt-2">There are <span className="gradient-text-pink-purple">500,000+</span> pairs.</span>
          </h2>

          <p className="text-xl text-text-secondary mb-12 max-w-2xl mx-auto font-body">
            If your token has a pool on any supported DEX -- on any EVM chain -- it can have a ZK-verified price feed. No application, no vendor contract, no waiting.
          </p>
        </div>

        {/* Comparison cards */}
        <div className="grid grid-cols-2 gap-6 max-w-lg mx-auto mb-16">
          <div className="glass rounded-2xl p-6">
            <div className="text-5xl font-mono font-bold text-chainlink-blue mb-2">350</div>
            <div className="text-sm text-text-muted font-body">Chainlink feeds</div>
            <div className="mt-4 h-3 bg-white/5 rounded-full overflow-hidden">
              <div className="h-full w-[1%] bg-chainlink-blue rounded-full min-w-[6px]" />
            </div>
          </div>
          <div className="glass rounded-2xl p-6 glow-green">
            <div className="text-5xl font-mono font-bold text-neon-green mb-2">500K+</div>
            <div className="text-sm text-text-muted font-body">On-chain pairs</div>
            <div className="mt-4 h-3 bg-white/5 rounded-full overflow-hidden">
              <div className="h-full w-full bg-gradient-to-r from-neon-green to-neon-cyan rounded-full" />
            </div>
          </div>
        </div>

        {/* Token grid */}
        <div className="grid grid-cols-6 sm:grid-cols-9 md:grid-cols-12 gap-2.5 max-w-4xl mx-auto mb-12">
          {tokens.map((t, i) => (
            <div
              key={i}
              className={`aspect-square rounded-xl flex items-center justify-center text-[10px] sm:text-xs font-mono font-bold transition-all duration-300 cursor-pointer ${
                t.lit
                  ? "bg-neon-green/15 border border-neon-green/40 text-neon-green hover:scale-110"
                  : "glass text-white/10 hover:bg-neon-green/10 hover:border-neon-green/30 hover:text-neon-green hover:scale-105"
              }`}
            >
              {t.s}
            </div>
          ))}
        </div>

        {/* Categories */}
        <div className="flex flex-wrap items-center justify-center gap-3 max-w-3xl mx-auto mb-12">
          {categories.map((cat) => (
            <span
              key={cat}
              className="glass rounded-lg px-4 py-2 text-sm text-text-secondary font-body"
            >
              {cat}
            </span>
          ))}
        </div>

        {/* Supported DEXes */}
        <div className="glass-strong rounded-2xl p-8 max-w-3xl mx-auto text-center">
          <p className="text-xs font-mono text-text-muted tracking-widest mb-5">SUPPORTED DEX PROTOCOLS (ANY EVM CHAIN)</p>
          <div className="flex flex-wrap items-center justify-center gap-3">
            {dexes.map((d) => (
              <div key={d.name} className="glass rounded-lg px-4 py-2.5">
                <span className="text-text-primary font-bold font-body text-sm">{d.name}</span>
                <span className="text-text-muted text-xs font-mono ml-2">{d.chains}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
