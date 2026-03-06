export default function Footer() {
  return (
    <footer className="py-28 px-6 relative overflow-hidden">
      {/* Vaporwave grid */}
      <div className="absolute bottom-0 left-0 right-0 h-[40vh] overflow-hidden opacity-20" aria-hidden="true">
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `
              linear-gradient(rgba(57,255,20,0.5) 1px, transparent 1px),
              linear-gradient(90deg, rgba(0,229,255,0.5) 1px, transparent 1px)
            `,
            backgroundSize: "60px 60px",
            transform: "perspective(500px) rotateX(60deg)",
            transformOrigin: "center top",
          }}
        />
      </div>

      <div className="max-w-4xl mx-auto text-center relative z-10">
        <h2 className="text-4xl sm:text-6xl font-display tracking-wide text-text-primary mb-2">
          <span className="block">Deploy a feed in</span>
          <span className="block gradient-text-green-cyan mt-2">5 minutes.</span>
        </h2>

        {/* Code block */}
        <div className="glass-strong rounded-2xl overflow-hidden text-left max-w-2xl mx-auto mb-12 mt-10 glow-green">
          <div className="flex items-center gap-3 px-6 py-3.5 border-b border-white/10">
            <div className="flex gap-2" aria-hidden="true">
              <div className="w-3.5 h-3.5 rounded-full bg-neon-pink/70" />
              <div className="w-3.5 h-3.5 rounded-full bg-neon-yellow/70" />
              <div className="w-3.5 h-3.5 rounded-full bg-neon-green/70" />
            </div>
            <span className="text-text-muted text-sm font-mono tracking-wider">terminal</span>
          </div>
          <pre className="p-6 sm:p-8 overflow-x-auto text-sm sm:text-base leading-relaxed font-mono">
            <code>
              <span className="text-neon-green">$</span>
              <span className="text-text-primary">
                {" "}
                git clone https://github.com/boundless-xyz/zk-oracle
              </span>
              {"\n"}
              <span className="text-neon-green">$</span>
              <span className="text-text-primary"> cd zk-oracle</span>
              {"\n\n"}
              <span className="text-text-muted">
                # Set your pool address and run
              </span>
              {"\n"}
              <span className="text-neon-green">$</span>
              <span className="text-text-primary">
                {" "}
                POOL_ADDRESS=0x... RPC_URL=https://... cargo run --release
              </span>
            </code>
          </pre>
        </div>

        {/* Links */}
        <div className="flex items-center justify-center gap-4 flex-wrap">
          <a
            href="https://github.com/boundless-xyz/zk-oracle"
            target="_blank"
            rel="noopener noreferrer"
            className="px-8 py-4 rounded-xl font-bold text-lg text-bg transition-all duration-300 hover:-translate-y-1 glow-green"
            style={{ background: "linear-gradient(135deg, #39ff14, #00e5ff)" }}
          >
            GitHub
          </a>
          <a
            href="https://docs.beboundless.xyz"
            target="_blank"
            rel="noopener noreferrer"
            className="glass px-8 py-4 rounded-xl text-text-primary font-bold text-lg hover:bg-white/10 transition-all duration-300 hover:-translate-y-1"
          >
            Docs
          </a>
          <a
            href="https://beboundless.xyz"
            target="_blank"
            rel="noopener noreferrer"
            className="glass px-8 py-4 rounded-xl text-text-primary font-bold text-lg hover:bg-white/10 transition-all duration-300 hover:-translate-y-1"
          >
            Boundless
          </a>
        </div>

        <div className="mt-24 pt-8 border-t border-white/10">
          <p className="text-text-muted text-sm font-body">
            Built with{" "}
            <span className="text-neon-green font-bold">Steel</span> +{" "}
            <span className="text-neon-cyan font-bold">RISC Zero</span> +{" "}
            <span className="text-neon-magenta font-bold">Boundless</span>
          </p>
        </div>
      </div>
    </footer>
  );
}
