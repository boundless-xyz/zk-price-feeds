import Reveal from "./Reveal";

const categories = [
  "Micro-cap DeFi tokens",
  "LP tokens & vault shares",
  "New launches (day-one feeds)",
  "Community tokens",
  "Cross-chain bridged assets",
  "RWA tokens without CEX listings",
];

const dexes = [
  { name: "Uniswap V2/V3", note: "observe() TWAP" },
  { name: "Curve", note: "StableSwap" },
  { name: "Balancer V2", note: "Weighted" },
  { name: "Solidly / Aerodrome", note: "Stable" },
  { name: "Trader Joe LB", note: "Active bin" },
  { name: "DODO PMM", note: "Mid price" },
  { name: "Algebra V3", note: "Camelot" },
  { name: "Any V2 fork", note: "SushiSwap, PancakeSwap, ..." },
];

export default function Problem() {
  return (
    <section className="py-20 sm:py-28 border-t border-border">
      <div className="max-w-5xl mx-auto px-5 sm:px-8">
        <Reveal>
          <div className="mb-12 sm:mb-16">
            <h2 className="text-3xl sm:text-4xl lg:text-5xl font-display font-bold leading-tight tracking-tight text-text-primary mb-5">
              Chainlink covers ~350 tokens.
            </h2>
            <p className="text-2xl sm:text-3xl font-display font-bold text-text-muted tracking-tight mb-5">
              There are 500,000+ pairs.
            </p>
            <p className="text-base sm:text-lg text-text-secondary max-w-lg font-body leading-relaxed">
              Want a feed for your token? Get in line. Or pay $50-200K/year to
              sponsor one. We built something different.
            </p>
          </div>
        </Reveal>

        {/* Proportional comparison */}
        <Reveal>
          <div className="mb-14 max-w-md">
            <div className="space-y-4">
              <div>
                <div className="flex items-baseline justify-between mb-1.5">
                  <span className="text-sm font-body text-text-muted">Chainlink feeds</span>
                  <span className="text-sm font-mono font-bold text-text-muted">~350</span>
                </div>
                <div className="h-2.5 bg-bg-raised rounded-full overflow-hidden">
                  <div className="h-full w-[0.07%] min-w-[3px] bg-text-muted/40 rounded-full" />
                </div>
              </div>
              <div>
                <div className="flex items-baseline justify-between mb-1.5">
                  <span className="text-sm font-body text-text-secondary">On-chain trading pairs</span>
                  <span className="text-sm font-mono font-bold text-accent">500,000+</span>
                </div>
                <div className="h-2.5 bg-bg-raised rounded-full overflow-hidden">
                  <div className="h-full w-full bg-accent/30 rounded-full" />
                </div>
              </div>
            </div>
          </div>
        </Reveal>

        {/* Token categories */}
        <Reveal>
          <p className="text-xs font-body font-medium text-text-muted mb-3 uppercase tracking-wider">
            Tokens Chainlink will never support
          </p>
          <div className="flex flex-wrap gap-2 mb-14">
            {categories.map((cat) => (
              <span
                key={cat}
                className="bg-bg-surface rounded-md px-3 py-1.5 text-sm text-text-secondary font-body"
              >
                {cat}
              </span>
            ))}
          </div>
        </Reveal>

        {/* Supported DEXes */}
        <Reveal>
          <p className="text-xs font-body font-medium text-text-muted mb-4 uppercase tracking-wider">
            8 DEX types supported
          </p>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
            {dexes.map((d) => (
              <div
                key={d.name}
                className="bg-bg-surface rounded-lg px-3 py-2.5 hover:bg-bg-raised transition-colors"
              >
                <span className="text-text-primary font-medium font-body text-sm block">
                  {d.name}
                </span>
                <span className="text-text-muted text-xs font-mono">
                  {d.note}
                </span>
              </div>
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  );
}
