import Reveal from "./Reveal";

const providers = [
  {
    name: "ZK Price Feed",
    coverage: "Any DEX pool",
    access: "Permissionless",
    trust: "Mathematics",
    color: "text-accent",
    highlight: true,
  },
  {
    name: "Chainlink",
    coverage: "~860 feeds",
    access: "Contact sales",
    trust: "4-31 node committee",
    color: "text-chainlink-blue",
  },
  {
    name: "Pyth",
    coverage: "~500 feeds",
    access: "Council approval",
    trust: "Wormhole guardians",
    color: "text-pyth-purple",
  },
  {
    name: "RedStone",
    coverage: "~1,000 feeds",
    access: "Team approval",
    trust: "Signer set",
    color: "text-redstone-red",
  },
  {
    name: "Chronicle",
    coverage: "~70 feeds",
    access: "Allowlist only",
    trust: "Schnorr multisig",
    color: "text-chronicle-teal",
  },
];

export default function CostComparison() {
  return (
    <section id="cost" className="py-20 sm:py-28 border-t border-border">
      <div className="max-w-5xl mx-auto px-5 sm:px-8">
        <Reveal>
          <h2 className="text-3xl sm:text-4xl font-display font-bold tracking-tight text-text-primary mb-3">
            Other oracles choose which tokens get a feed.
          </h2>
          <p className="text-text-secondary font-body mb-10 sm:mb-14 max-w-lg leading-relaxed">
            Existing oracles curate a fixed list of pairs and gatekeep access
            behind vendor contracts, governance votes, or allowlists. If your
            token isn&apos;t on their list, you&apos;re out of luck.
          </p>
        </Reveal>

        {/* Full-width table + right-aligned callouts */}
        <div className="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-8 items-start">
          {/* Table — clean, no card wrapper */}
          <Reveal>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left py-3 px-4 text-[11px] font-body font-medium text-text-muted uppercase tracking-wider">
                      Provider
                    </th>
                    <th className="text-left py-3 px-4 text-[11px] font-body font-medium text-text-muted uppercase tracking-wider">
                      Pair coverage
                    </th>
                    <th className="text-left py-3 px-4 text-[11px] font-body font-medium text-text-muted uppercase tracking-wider hidden sm:table-cell">
                      Access
                    </th>
                    <th className="text-left py-3 px-4 text-[11px] font-body font-medium text-text-muted uppercase tracking-wider">
                      Trust model
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {providers.map((p) => (
                    <tr
                      key={p.name}
                      className={`border-b border-border/50 last:border-b-0 ${
                        p.highlight ? "bg-accent/5" : ""
                      }`}
                    >
                      <td
                        className={`py-3.5 px-4 font-semibold font-body text-sm ${p.color}`}
                      >
                        {p.name}
                      </td>
                      <td className="py-3.5 px-4 text-sm font-mono">
                        {p.highlight ? (
                          <span className="text-accent font-bold">
                            {p.coverage}
                          </span>
                        ) : (
                          <span className="text-text-secondary">
                            {p.coverage}
                          </span>
                        )}
                      </td>
                      <td className="py-3.5 px-4 text-sm font-body hidden sm:table-cell">
                        {p.highlight ? (
                          <span className="text-accent font-semibold">
                            {p.access}
                          </span>
                        ) : (
                          <span className="text-text-muted">{p.access}</span>
                        )}
                      </td>
                      <td className="py-3.5 px-4 text-sm">
                        {p.highlight ? (
                          <span className="font-semibold text-accent font-mono">
                            {p.trust}
                          </span>
                        ) : (
                          <span className="text-text-muted font-body">
                            {p.trust}
                          </span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Reveal>

          {/* Right-aligned callout cards */}
          <div className="flex flex-col gap-4">
            <Reveal>
              <div className="border-l-4 border-accent bg-white rounded-r-lg p-5">
                <p className="font-mono text-2xl font-bold text-text-primary mb-1">
                  500K+
                </p>
                <p className="text-text-secondary font-body text-sm leading-relaxed">
                  DEX pools exist on-chain. Every one of them is a feed you can
                  deploy today — no proposal, no vote, no vendor.
                </p>
              </div>
            </Reveal>
            <Reveal>
              <div className="border-l-4 border-accent bg-white rounded-r-lg p-5">
                <p className="font-mono text-2xl font-bold text-text-primary mb-1">
                  ~$0.004
                </p>
                <p className="text-text-secondary font-body text-sm leading-relaxed">
                  Per ZK-proven update via the Boundless proving network. No LINK
                  subsidies. No hidden vendor contracts. Transparent cost.
                </p>
              </div>
            </Reveal>
            <Reveal>
              <div className="border-l-4 border-accent bg-white rounded-r-lg p-5">
                <p className="font-mono text-2xl font-bold text-accent mb-1">
                  Math
                </p>
                <p className="text-text-secondary font-body text-sm leading-relaxed">
                  ZK proofs verify prices from real on-chain state. No committee.
                  No multisig. No guardians. A proof anyone can verify.
                </p>
              </div>
            </Reveal>
          </div>
        </div>
      </div>
    </section>
  );
}
