import Reveal from "./Reveal";

const steps = [
  {
    num: "01",
    title: "Read",
    description:
      "Steel reads DEX pool state across 10+ blocks inside the RISC Zero zkVM. V3 pools use observe() TWAP -- a flash-loan-immune geometric mean price from tick accumulators. V2 pools read getReserves(). Every call is cryptographically tied to actual on-chain state.",
    detail: "8 DEX types supported",
  },
  {
    num: "02",
    title: "Prove",
    description:
      "Boundless computes the median-of-TWAP price and generates a ZK proof. 73 million cycles. $0.004 on the Boundless marketplace. Proven in 15 seconds.",
    detail: "Outsourced to Boundless provers",
  },
  {
    num: "03",
    title: "Verify",
    description:
      "A Solidity contract checks the proof on-chain (~250K gas) and stores the price. Safety checks enforce minimum samples, deviation limits, and block freshness. Your protocol calls latestRoundData() -- zero code changes.",
    detail: "Chainlink-compatible interface",
  },
];

export default function HowItWorks() {
  return (
    <section id="how-it-works" className="py-20 sm:py-28 bg-bg-surface">
      <div className="max-w-5xl mx-auto px-5 sm:px-8">
        <Reveal>
          <h2 className="text-3xl sm:text-4xl font-display font-bold tracking-tight text-text-primary mb-12 sm:mb-16">
            Three steps. That&apos;s it.
          </h2>
        </Reveal>

        <div className="space-y-4 sm:space-y-0 sm:grid sm:grid-cols-3 sm:gap-6">
          {steps.map((step, i) => (
            <Reveal key={step.num} delay={i * 100}>
              <div className="bg-white rounded-lg p-6 sm:p-7 border border-border h-full hover:border-border-strong hover:-translate-y-0.5 transition-all duration-200">
                <div className="flex items-center gap-3 mb-4">
                  <span
                    className={`inline-flex items-center justify-center w-8 h-8 rounded-full text-xs font-mono font-bold ${
                      i === 0
                        ? "bg-accent text-white"
                        : "bg-bg-raised text-text-secondary"
                    }`}
                  >
                    {step.num}
                  </span>
                  <h3 className="text-xl font-display font-bold text-text-primary">
                    {step.title}
                  </h3>
                </div>
                <p className="text-text-secondary leading-relaxed font-body text-sm mb-4">
                  {step.description}
                </p>
                <p className="text-xs font-mono text-accent font-semibold">
                  {step.detail}
                </p>
              </div>
            </Reveal>
          ))}
        </div>

        {/* Security callout */}
        <Reveal>
          <div className="mt-10 sm:mt-14 bg-white rounded-lg border border-border p-6 sm:p-7 max-w-2xl">
            <div className="flex items-start gap-3">
              <span className="inline-flex items-center justify-center w-8 h-8 rounded-full bg-accent-dim text-accent shrink-0 mt-0.5">
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 16 16"
                  fill="none"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <path
                    d="M8 1L2 4v4c0 3.5 2.5 6.4 6 7 3.5-.6 6-3.5 6-7V4L8 1z"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinejoin="round"
                  />
                  <path
                    d="M6 8l1.5 1.5L10 6"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </span>
              <div>
                <h3 className="text-base font-display font-bold text-text-primary mb-1.5">
                  TWAP + Median. Can&apos;t flash-loan that.
                </h3>
                <p className="text-text-secondary font-body text-sm leading-relaxed mb-3">
                  V3 pools use{" "}
                  <span className="font-mono text-text-primary">
                    observe()
                  </span>{" "}
                  TWAP accumulators that only update once per block -- flash
                  loans cannot manipulate them. Then we take the{" "}
                  <span className="text-accent font-semibold">
                    median across 10+ blocks
                  </span>
                  . Double protection.
                </p>
                <p className="text-text-secondary font-body text-sm leading-relaxed">
                  To manipulate this feed, an attacker would need to sustain
                  manipulation across &gt;50% of sampled blocks while risking
                  arbitrageurs correcting the price every block.
                </p>
              </div>
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  );
}
