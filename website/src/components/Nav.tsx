export default function Nav() {
  return (
    <nav className="fixed top-0 left-0 right-0 z-50 glass-strong">
      <div className="max-w-6xl mx-auto px-6 py-3 flex items-center justify-between">
        <a href="#" className="font-display text-xl tracking-wide gradient-text-green-cyan">
          CheapOracles
        </a>
        <div className="hidden sm:flex items-center gap-6 text-sm font-body">
          <a href="#live-feeds" className="text-text-secondary hover:text-neon-green transition-colors">Feeds</a>
          <a href="#how-it-works" className="text-text-secondary hover:text-neon-green transition-colors">How It Works</a>
          <a href="#cost" className="text-text-secondary hover:text-neon-green transition-colors">Cost</a>
          <a
            href="https://github.com/boundless-xyz/zk-oracle"
            target="_blank"
            rel="noopener noreferrer"
            className="px-4 py-1.5 rounded-lg text-bg font-bold text-sm transition-all hover:-translate-y-0.5"
            style={{ background: "linear-gradient(135deg, #39ff14, #00e5ff)" }}
          >
            GitHub
          </a>
        </div>
      </div>
    </nav>
  );
}
