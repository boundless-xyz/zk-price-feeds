import Reveal from "./Reveal";

const computations = [
  {
    name: "Single pool median",
    desc: "20-block median from any DEX pool with TWAP for V3",
    tag: "Current",
    current: true,
  },
  {
    name: "Multi-pool aggregation",
    desc: "Read 5 DEX pools across protocols, liquidity-weighted median",
    tag: "325K gas",
    current: false,
  },
  {
    name: "Cross-DEX price",
    desc: "Aggregate Uniswap + Sushi + Curve into a single feed",
    tag: "325K gas",
    current: false,
  },
  {
    name: "Composite index",
    desc: "Top-10 DeFi token index verified in one proof",
    tag: "325K gas",
    current: false,
  },
  {
    name: "LP token pricing",
    desc: "Fair value from underlying reserves, proven on-chain",
    tag: "325K gas",
    current: false,
  },
  {
    name: "On-chain volatility",
    desc: "Implied vol from price variance across blocks",
    tag: "325K gas",
    current: false,
  },
];

export default function BeyondBasic() {
  return (
    <section className="py-20 sm:py-28 bg-bg-surface">
      <div className="max-w-5xl mx-auto px-5 sm:px-8">
        <Reveal>
          <h2 className="text-3xl sm:text-4xl font-display font-bold tracking-tight text-text-primary mb-3">
            This is the simplest thing we can prove.
          </h2>
          <p className="text-text-secondary font-body mb-10 sm:mb-14 max-w-xl leading-relaxed">
            Reading prices from one pool is day-one functionality. The same
            framework proves arbitrarily complex computations -- all at the same
            on-chain gas cost.
          </p>
        </Reveal>

        <Reveal>
          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-3 sm:gap-4 mb-10">
            {computations.map((c) => (
              <div
                key={c.name}
                className={`rounded-lg p-5 sm:p-6 ${
                  c.current
                    ? "bg-accent-dim border border-accent/20"
                    : "bg-white border border-border hover:border-border-strong hover:-translate-y-0.5 transition-all duration-200"
                }`}
              >
                <div className="flex items-start justify-between mb-2">
                  <h3 className="font-semibold text-text-primary font-body">
                    {c.name}
                  </h3>
                  <span
                    className={`text-[10px] font-mono px-2 py-0.5 rounded shrink-0 ml-2 ${
                      c.current
                        ? "bg-accent/15 text-accent font-semibold"
                        : "bg-bg-surface text-text-muted"
                    }`}
                  >
                    {c.tag}
                  </span>
                </div>
                <p className="text-sm text-text-secondary font-body">
                  {c.desc}
                </p>
              </div>
            ))}
          </div>
        </Reveal>

        <Reveal>
          <div className="bg-white border border-border rounded-lg p-5 sm:p-6 max-w-xl">
            <p className="text-base font-display font-bold text-text-primary mb-2">
              On-chain verification cost is fixed regardless of complexity.
            </p>
            <p className="text-text-secondary font-body text-sm leading-relaxed">
              Whether the prover reads 1 pool or 50 pools, the proof is the same
              size. Only off-chain proving cost scales -- and at{" "}
              <span className="text-accent font-mono font-semibold">
                $0.05/billion cycles
              </span>
              , a 10x bigger computation costs{" "}
              <span className="text-accent font-mono font-semibold">
                $0.04
              </span>
              .
            </p>
          </div>
        </Reveal>
      </div>
    </section>
  );
}
