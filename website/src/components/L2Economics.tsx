import Reveal from "./Reveal";

const rows = [
  { freq: "Every 1 min", l1: "$145,818", l2: "$450" },
  { freq: "Every 5 min", l1: "$29,164", l2: "$90" },
  { freq: "Every 1 hour", l1: "$2,430", l2: "$7.50" },
  { freq: "Every 24 hours", l1: "$101", l2: "$0.31" },
];

export default function L2Economics() {
  return (
    <section className="py-20 sm:py-28 border-t border-border">
      <div className="max-w-5xl mx-auto px-5 sm:px-8">
        <Reveal>
          <h2 className="text-3xl sm:text-4xl font-display font-bold tracking-tight text-text-primary mb-3">
            On L2, this gets ridiculous.
          </h2>
          <p className="text-text-secondary font-body mb-10 sm:mb-14 max-w-lg leading-relaxed">
            On L2, proving cost (<span className="font-mono">$0.004</span>) dominates gas cost (<span className="font-mono">$0.007</span>). A
            continuously-updated oracle feed costs less than a cup of coffee per
            month.
          </p>
        </Reveal>

        {/* Asymmetric split — table left, latency cards right */}
        <div className="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-6 lg:gap-8">
          {/* Table on left */}
          <Reveal>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left py-3 px-4 text-[11px] font-body font-medium text-text-muted uppercase tracking-wider">
                      Frequency
                    </th>
                    <th className="text-right py-3 px-4 text-[11px] font-body font-medium text-text-muted uppercase tracking-wider">
                      L1 cost/month
                    </th>
                    <th className="text-right py-3 px-4 text-[11px] font-body font-medium text-text-muted uppercase tracking-wider">
                      L2 cost/month
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r) => (
                    <tr
                      key={r.freq}
                      className="border-b border-border/50 last:border-b-0"
                    >
                      <td className="py-3.5 px-4 text-sm font-body text-text-primary">
                        {r.freq}
                      </td>
                      <td className="py-3.5 px-4 text-sm font-mono text-text-muted text-right">
                        {r.l1}
                      </td>
                      <td className="py-3.5 px-4 text-sm font-mono text-accent font-bold text-right">
                        {r.l2}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Reveal>

          {/* Latency cards on right — stacked */}
          <div className="flex flex-col gap-4">
            <Reveal>
              <div className="bg-white border border-border rounded-xl p-5">
                <div className="font-mono text-3xl font-bold text-accent mb-1">
                  ~16s
                </div>
                <div className="text-sm text-text-secondary font-body">
                  Update latency on L2
                </div>
                <div className="text-xs text-text-muted font-mono mt-1">
                  15s proving + ~1s block time
                </div>
              </div>
            </Reveal>
            <Reveal delay={100}>
              <div className="bg-white border border-border rounded-xl p-5">
                <div className="font-mono text-3xl font-bold text-text-primary mb-1">
                  ~27s
                </div>
                <div className="text-sm text-text-secondary font-body">
                  Update latency on L1
                </div>
                <div className="text-xs text-text-muted font-mono mt-1">
                  15s proving + 12s block time
                </div>
              </div>
            </Reveal>
          </div>
        </div>
      </div>
    </section>
  );
}
