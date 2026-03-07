import Reveal from "./Reveal";

const providers = [
  {
    name: "Chainlink",
    cost: "$0 (LINK-subsidized)",
    trust: "4-31 node committee",
    gas: "~400K",
    vendor: "\"Contact sales\"",
    color: "text-chainlink-blue",
  },
  {
    name: "Pyth",
    cost: "$0.001/update",
    trust: "Wormhole guardians",
    gas: "~90K",
    vendor: "Free",
    color: "text-pyth-purple",
  },
  {
    name: "RedStone",
    cost: "$0 (sponsor-paid)",
    trust: "Signer set",
    gas: "~65K",
    vendor: "Free",
    color: "text-redstone-red",
  },
  {
    name: "Chronicle",
    cost: "$0 (sponsor-paid)",
    trust: "Schnorr multisig",
    gas: "~250K",
    vendor: "Allowlist only",
    color: "text-chronicle-teal",
  },
  {
    name: "ZK Price Feed",
    cost: "$0.004 + gas",
    trust: "Mathematics",
    gas: "~250K",
    vendor: "Free, permissionless",
    color: "text-accent",
    highlight: true,
  },
];

export default function CostComparison() {
  return (
    <section id="cost" className="py-20 sm:py-28 bg-bg-surface">
      <div className="max-w-5xl mx-auto px-5 sm:px-8">
        <Reveal>
          <h2 className="text-3xl sm:text-4xl font-display font-bold tracking-tight text-text-primary mb-3">
            The oracle pricing racket, visualized.
          </h2>
          <p className="text-text-secondary font-body mb-10 sm:mb-14 max-w-lg leading-relaxed">
            Other oracles hide their costs behind vendor contracts and
            governance votes. Here&apos;s the real comparison.
          </p>
        </Reveal>

        {/* Table */}
        <Reveal>
          <div className="bg-white border border-border-strong rounded-lg overflow-hidden mb-8">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left py-3 px-4 sm:px-5 text-xs font-body font-medium text-text-muted">
                      Provider
                    </th>
                    <th className="text-left py-3 px-4 sm:px-5 text-xs font-body font-medium text-text-muted hidden sm:table-cell">
                      Update cost
                    </th>
                    <th className="text-left py-3 px-4 sm:px-5 text-xs font-body font-medium text-text-muted hidden md:table-cell">
                      On-chain gas
                    </th>
                    <th className="text-left py-3 px-4 sm:px-5 text-xs font-body font-medium text-text-muted hidden lg:table-cell">
                      Vendor cost
                    </th>
                    <th className="text-left py-3 px-4 sm:px-5 text-xs font-body font-medium text-text-muted">
                      Trust model
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {providers.map((p) => (
                    <tr
                      key={p.name}
                      className={`border-b border-border/50 last:border-b-0 ${
                        p.highlight ? "bg-accent-dim" : ""
                      }`}
                    >
                      <td
                        className={`py-3.5 px-4 sm:px-5 font-semibold font-body text-sm ${p.color}`}
                      >
                        {p.name}
                      </td>
                      <td className="py-3.5 px-4 sm:px-5 text-sm font-mono text-text-secondary hidden sm:table-cell">
                        {p.cost}
                      </td>
                      <td className="py-3.5 px-4 sm:px-5 text-sm font-mono text-text-secondary hidden md:table-cell">
                        {p.gas}
                      </td>
                      <td className="py-3.5 px-4 sm:px-5 text-sm text-text-secondary font-body hidden lg:table-cell">
                        {p.vendor}
                      </td>
                      <td className="py-3.5 px-4 sm:px-5 text-sm">
                        {p.highlight ? (
                          <span className="text-accent font-bold font-mono">
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
          </div>
        </Reveal>

        {/* Mathematics callout */}
        <Reveal>
          <div className="border-l-4 border-accent bg-white rounded-r-lg p-5 sm:p-6 max-w-xl">
            <p className="text-xl sm:text-2xl font-display font-bold text-text-primary mb-2">
              The only oracle where the trust model is &ldquo;mathematics.&rdquo;
            </p>
            <p className="text-text-secondary font-body text-sm leading-relaxed">
              ZK proofs verify that the price was computed correctly from real
              on-chain state. No committee of signers. No multisig. No
              guardians. Just a proof anyone can verify.
            </p>
          </div>
        </Reveal>
      </div>
    </section>
  );
}
