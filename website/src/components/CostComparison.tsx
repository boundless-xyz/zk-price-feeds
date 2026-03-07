const providers = [
  { name: "Chainlink", provingCost: "$0 (LINK-subsidized)", trust: "4-31 node committee", gas: "~400K", color: "text-chainlink-blue", highlight: false },
  { name: "Pyth", provingCost: "$0.001/update", trust: "Wormhole guardians", gas: "~65K", color: "text-pyth-purple", highlight: false },
  { name: "RedStone", provingCost: "$0 (sponsor-paid)", trust: "Signer set", gas: "~90K", color: "text-redstone-red", highlight: false },
  { name: "Chronicle", provingCost: "$0 (sponsor-paid)", trust: "Schnorr multisig", gas: "~150K", color: "text-chronicle-teal", highlight: false },
  { name: "ZK Oracle", provingCost: "$0.004 + gas", trust: "Mathematics", gas: "~250K", color: "text-neon-green", highlight: true },
];

export default function CostComparison() {
  return (
    <section id="cost" className="py-28 px-6 relative overflow-hidden">
      <div className="absolute bottom-0 left-1/2 -translate-x-1/2 w-[600px] h-[400px] bg-neon-purple/5 rounded-full blur-[200px]" />

      <div className="max-w-6xl mx-auto relative z-10">
        <div className="text-center mb-16">
          <h2 className="text-4xl sm:text-6xl font-display tracking-wide">
            <span className="text-text-primary block">And it&apos;s cheaper than</span>
            <span className="gradient-text-cyan-purple block text-3xl sm:text-5xl mt-2">what you&apos;re paying now.</span>
          </h2>
        </div>

        {/* Provider comparison table */}
        <div className="glass rounded-2xl overflow-hidden mb-12 max-w-4xl mx-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-white/10">
                <th className="text-left py-4 px-5 text-xs font-mono text-text-muted tracking-widest">PROVIDER</th>
                <th className="text-left py-4 px-5 text-xs font-mono text-text-muted tracking-widest">UPDATE COST</th>
                <th className="text-left py-4 px-5 text-xs font-mono text-text-muted tracking-widest">ON-CHAIN GAS</th>
                <th className="text-left py-4 px-5 text-xs font-mono text-text-muted tracking-widest">TRUST</th>
              </tr>
            </thead>
            <tbody>
              {providers.map((p) => (
                <tr
                  key={p.name}
                  className={`border-b border-white/5 ${p.highlight ? "" : "hover:bg-white/[0.03]"} transition-colors`}
                  style={p.highlight ? { background: "linear-gradient(135deg, rgba(57,255,20,0.05), rgba(0,229,255,0.03))" } : undefined}
                >
                  <td className={`py-4 px-5 font-bold font-body ${p.color}`}>{p.name}</td>
                  <td className="py-4 px-5 text-sm font-mono">
                    <span className={p.highlight ? "text-neon-green font-bold" : "text-text-secondary"}>{p.provingCost}</span>
                  </td>
                  <td className="py-4 px-5 text-sm font-mono">
                    <span className={p.highlight ? "text-neon-green font-bold" : "text-text-secondary"}>{p.gas}</span>
                  </td>
                  <td className="py-4 px-5 text-sm">
                    {p.highlight ? (
                      <span className="inline-flex items-center px-3 py-1.5 rounded-full bg-neon-green/10 border border-neon-green/30 text-neon-green font-bold font-mono text-xs tracking-wider">{p.trust}</span>
                    ) : (
                      <span className="text-text-muted font-body">{p.trust}</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Verification cost callout */}
        <div className="glass-strong rounded-2xl p-8 max-w-3xl mx-auto text-center">
          <p className="text-text-secondary font-body text-lg">
            Chainlink verifies BLS aggregate signatures on-chain at{" "}
            <span className="text-chainlink-blue font-mono font-bold">~400K gas</span>.
            Chronicle uses Schnorr multisigs at{" "}
            <span className="text-chronicle-teal font-mono font-bold">~150K gas</span>.
            A ZK proof verifies at{" "}
            <span className="text-neon-green font-mono font-bold">~250K gas</span>{" "}
            -- cheaper than BLS, with no committee to trust.
          </p>
        </div>
      </div>
    </section>
  );
}
