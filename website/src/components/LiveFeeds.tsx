const feeds = [
  { token: "RPL", price: "$2.38", dex: "Uniswap", hasChainlink: false },
  { token: "SHIB", price: "$0.0000055", dex: "Uniswap", hasChainlink: false },
  { token: "CAKE", price: "$2.41", dex: "PancakeSwap V2", hasChainlink: false },
  { token: "AERO", price: "$0.87", dex: "Aerodrome CL", hasChainlink: false },
  { token: "ETH", price: "$2,075", dex: "Uniswap V3", hasChainlink: true },
  { token: "UNI", price: "$3.97", dex: "Uniswap V2", hasChainlink: true },
  { token: "LINK", price: "$9.19", dex: "Balancer V2", hasChainlink: true },
  { token: "AAVE", price: "$116.94", dex: "Curve", hasChainlink: true },
];

export default function LiveFeeds() {
  return (
    <section id="live-feeds" className="py-28 px-6 relative overflow-hidden">
      <div className="absolute inset-0 opacity-[0.02]" aria-hidden="true"
        style={{
          backgroundImage: `linear-gradient(rgba(0,229,255,0.8) 1px, transparent 1px), linear-gradient(90deg, rgba(0,229,255,0.8) 1px, transparent 1px)`,
          backgroundSize: "40px 40px",
        }}
      />

      <div className="max-w-6xl mx-auto relative z-10">
        <div className="text-center mb-16">
          <h2 className="text-4xl sm:text-5xl lg:text-6xl font-display tracking-wide mb-4">
            <span className="gradient-text-green-cyan">Live feeds.</span>{" "}
            <span className="text-text-primary">Any DEX.</span>
          </h2>
          <p className="text-xl text-text-secondary font-body max-w-2xl mx-auto">
            Uniswap V2/V3, PancakeSwap, Curve, Balancer, Aerodrome, Trader Joe, DODO -- 10 DEX standards across every EVM chain.
          </p>
        </div>

        {/* Table */}
        <div className="glass rounded-2xl overflow-hidden">
          <table className="w-full">
            <thead>
              <tr className="border-b border-white/10">
                <th className="text-left py-4 px-6 text-sm font-mono text-text-muted tracking-wider">TOKEN</th>
                <th className="text-left py-4 px-6 text-sm font-mono text-text-muted tracking-wider hidden sm:table-cell">DEX</th>
                <th className="text-right py-4 px-6 text-sm font-mono text-text-muted tracking-wider">PRICE</th>
                <th className="text-right py-4 px-6 text-sm font-mono text-text-muted tracking-wider hidden md:table-cell">COST</th>
                <th className="text-right py-4 px-6 text-sm font-mono text-text-muted tracking-wider">CHAINLINK?</th>
              </tr>
            </thead>
            <tbody>
              {/* Your token row */}
              <tr className="border-b-2 border-neon-green/30" style={{ background: "linear-gradient(135deg, rgba(57,255,20,0.05), rgba(0,229,255,0.05))" }}>
                <td className="py-4 px-6"><span className="font-display text-neon-green text-lg tracking-wide">Your token</span></td>
                <td className="py-4 px-6 font-mono text-neon-cyan italic hidden sm:table-cell">Any supported DEX</td>
                <td className="py-4 px-6 text-right font-mono text-neon-cyan italic text-lg">Any price</td>
                <td className="py-4 px-6 text-right font-mono text-neon-green font-bold hidden md:table-cell">$0.01</td>
                <td className="py-4 px-6 text-right">
                  <span className="inline-flex items-center px-3 py-1 rounded-full bg-neon-green/15 border border-neon-green/40 text-neon-green text-sm font-bold font-mono">NO FEED NEEDED</span>
                </td>
              </tr>
              {feeds.map((feed) => (
                <tr key={feed.token} className="border-b border-white/5 hover:bg-white/5 transition-all duration-200">
                  <td className="py-3.5 px-6"><span className="font-bold text-text-primary font-body text-lg">{feed.token}</span></td>
                  <td className="py-3.5 px-6 font-mono text-text-secondary text-sm hidden sm:table-cell">{feed.dex}</td>
                  <td className="py-3.5 px-6 text-right font-mono text-text-primary text-lg">{feed.price}</td>
                  <td className="py-3.5 px-6 text-right font-mono text-neon-green hidden md:table-cell">$0.01</td>
                  <td className="py-3.5 px-6 text-right">
                    {feed.hasChainlink ? (
                      <span className="text-text-muted text-sm font-body">Has feed</span>
                    ) : (
                      <span className="inline-flex items-center px-3 py-1 rounded-full bg-neon-pink/10 border border-neon-pink/30 text-neon-pink text-sm font-bold font-mono">NO FEED</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <p className="text-center text-text-muted mt-8 text-sm font-mono tracking-wider">
          $0.004 PROVING + $0.007 GAS ON L2 = $0.01 PER UPDATE
        </p>
      </div>
    </section>
  );
}
