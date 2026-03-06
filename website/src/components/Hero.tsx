export default function Hero() {
  return (
    <section className="relative min-h-screen flex items-center justify-center overflow-hidden pt-16">
      {/* Vaporwave grid floor */}
      <div className="absolute bottom-0 left-0 right-0 h-[60vh] overflow-hidden opacity-30 hidden md:block" aria-hidden="true">
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `
              linear-gradient(rgba(217,70,239,0.6) 1px, transparent 1px),
              linear-gradient(90deg, rgba(217,70,239,0.6) 1px, transparent 1px)
            `,
            backgroundSize: "60px 60px",
            transform: "perspective(500px) rotateX(60deg)",
            transformOrigin: "center top",
          }}
        />
      </div>

      {/* Gradient orbs */}
      <div className="absolute top-20 -left-40 w-[500px] h-[500px] bg-neon-purple/10 rounded-full blur-[150px]" />
      <div className="absolute bottom-20 -right-40 w-[500px] h-[500px] bg-neon-cyan/10 rounded-full blur-[150px]" />

      <div className="relative z-10 max-w-5xl mx-auto px-6 text-center">
        {/* Badge */}
        <div className="inline-flex items-center gap-2.5 px-5 py-2.5 rounded-full glass mb-10">
          <span className="w-2.5 h-2.5 rounded-full bg-neon-green animate-glow-pulse" />
          <span className="text-neon-green text-sm font-mono tracking-wider">
            LIVE ON ETHEREUM MAINNET
          </span>
        </div>

        {/* Main headline */}
        <h1 className="mb-8">
          <span className="block text-5xl sm:text-7xl lg:text-[5.5rem] font-display leading-[1.1] tracking-wide text-text-primary mb-2">
            A cost-efficient oracle
          </span>
          <span className="block text-5xl sm:text-7xl lg:text-[5.5rem] font-display leading-[1.1] tracking-wide text-text-primary mb-4">
            for any token.
          </span>
          <span className="block text-5xl sm:text-7xl lg:text-[5.5rem] font-display leading-[1.1] tracking-wide gradient-text-green-cyan">
            Trust math, not a multisig.
          </span>
        </h1>

        {/* Subhead -- tight, outcome-focused */}
        <p className="text-lg sm:text-xl text-text-secondary max-w-2xl mx-auto mb-12 leading-relaxed font-body">
          Any token with a DEX pool gets a ZK-verified price feed for $0.01 on L2.
          Compatible with existing oracle standards. No committees. No vendor contracts. No permission needed.
        </p>

        {/* CTAs */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4 mb-20">
          <a
            href="https://github.com/boundless-xyz/zk-oracle"
            target="_blank"
            rel="noopener noreferrer"
            className="group px-8 py-4 rounded-xl font-bold text-lg text-bg transition-all duration-300 hover:-translate-y-1 glow-green"
            style={{ background: "linear-gradient(135deg, #39ff14, #00e5ff)" }}
          >
            Deploy a Feed &rarr;
          </a>
          <a
            href="#live-feeds"
            className="px-8 py-4 glass rounded-xl text-text-primary font-bold text-lg hover:bg-white/10 transition-all duration-300 hover:-translate-y-1"
          >
            View Live Feeds &darr;
          </a>
        </div>

        {/* Stats bar -- 500K first (most differentiated) */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 max-w-3xl mx-auto">
          {[
            { value: "500K+", label: "possible pairs", color: "text-neon-magenta" },
            { value: "$0.01", label: "per update (L2)", color: "text-neon-green" },
            { value: "15s", label: "proving time", color: "text-neon-cyan" },
            { value: "$0", label: "vendor cost", color: "text-neon-yellow" },
          ].map((stat) => (
            <div key={stat.label} className="glass rounded-xl p-4 text-center hover:scale-105 transition-transform duration-300">
              <div className={`text-2xl sm:text-3xl font-mono font-bold ${stat.color}`}>
                {stat.value}
              </div>
              <div className="text-sm text-text-muted mt-1 font-body">{stat.label}</div>
            </div>
          ))}
        </div>
      </div>

      {/* Scroll indicator */}
      <div className="absolute bottom-8 left-1/2 -translate-x-1/2 animate-float" aria-hidden="true">
        <div className="w-6 h-10 rounded-full border-2 border-neon-purple/50 flex items-start justify-center p-1.5">
          <div className="w-1.5 h-3 rounded-full bg-neon-purple animate-glow-pulse" />
        </div>
      </div>
    </section>
  );
}
