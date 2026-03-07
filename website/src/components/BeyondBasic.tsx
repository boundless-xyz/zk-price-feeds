const computations = [
  { name: "Single pool median", desc: "20-block median from any V2-compatible pool", tag: "Current", gradient: "from-neon-green to-neon-cyan", tagBg: "bg-neon-green/10 text-neon-green border-neon-green/30" },
  { name: "Multi-pool aggregation", desc: "Read 5 DEX pools across protocols, liquidity-weighted median", tag: "325K gas", gradient: "from-neon-cyan to-neon-blue", tagBg: "bg-neon-cyan/10 text-neon-cyan border-neon-cyan/30" },
  { name: "Composite index", desc: "Top-10 DeFi token index verified in one proof", tag: "325K gas", gradient: "from-neon-purple to-neon-magenta", tagBg: "bg-neon-magenta/10 text-neon-magenta border-neon-magenta/30" },
  { name: "LP token pricing", desc: "Fair value from underlying reserves, proven on-chain", tag: "325K gas", gradient: "from-neon-magenta to-neon-pink", tagBg: "bg-neon-pink/10 text-neon-pink border-neon-pink/30" },
];

export default function BeyondBasic() {
  return (
    <section className="py-28 px-6 relative overflow-hidden">
      <div className="absolute top-1/2 left-0 w-[400px] h-[400px] bg-neon-purple/5 rounded-full blur-[200px]" />

      <div className="max-w-6xl mx-auto relative z-10">
        <div className="text-center mb-16">
          <h2 className="text-4xl sm:text-6xl font-display tracking-wide">
            <span className="text-text-primary block">This is the <span className="gradient-text-pink-purple">simplest thing</span></span>
            <span className="text-text-primary block">we can prove.</span>
          </h2>
        </div>

        <div className="grid sm:grid-cols-2 gap-5 max-w-4xl mx-auto mb-14">
          {computations.map((c, i) => (
            <div
              key={c.name}
              className={`glass-strong rounded-2xl p-6 transition-all duration-300 hover:-translate-y-1 hover:bg-white/[0.08] ${
                i === 0 ? "ring-2 ring-neon-green/20 glow-green" : ""
              }`}
            >
              <div className={`h-1 w-12 rounded-full bg-gradient-to-r ${c.gradient} mb-4`} />
              <div className="flex items-start justify-between mb-3">
                <h3 className="font-bold text-text-primary font-body text-lg">{c.name}</h3>
                <span className={`text-[10px] font-mono font-bold px-2.5 py-1 rounded-full border ${c.tagBg} tracking-wider shrink-0 ml-2`}>{c.tag}</span>
              </div>
              <p className="text-sm text-text-secondary font-body">{c.desc}</p>
            </div>
          ))}
        </div>

        {/* Key insight */}
        <div className="glass-strong rounded-2xl p-10 max-w-4xl mx-auto text-center glow-purple">
          <p className="text-2xl font-display tracking-wide text-text-primary mb-4">
            On-chain verification cost is <span className="gradient-text-cyan-purple">fixed</span> regardless of complexity.
          </p>
          <p className="text-text-secondary font-body text-lg">
            Whether the prover reads 1 pool or 50 pools, the proof is the same size. Only off-chain proving cost scales -- and at{" "}
            <span className="text-neon-green font-mono font-bold">$0.05/billion cycles</span>, a 10x bigger computation costs{" "}
            <span className="text-neon-green font-mono font-bold">$0.04</span>.
          </p>
        </div>
      </div>
    </section>
  );
}
