import { type ReactNode } from "react";
import Reveal from "./Reveal";

const steps: { num: string; title: string; description: ReactNode; detail: string }[] = [
  {
    num: "01",
    title: "Read",
    description: (
      <>
        Steel reads DEX pool state across a contiguous range of <span className="font-mono">10+</span> blocks inside the RISC Zero zkVM. The median price across the range is computed — manipulation of any single block has no effect. The range is configurable for stronger guarantees. Every read is cryptographically tied to actual on-chain state.
      </>
    ),
    detail: "8 DEX types supported",
  },
  {
    num: "02",
    title: "Prove",
    description: (
      <>
        Boundless computes the multi-block median price and generates a ZK proof. <span className="font-mono">73M</span> cycles. <span className="font-mono">$0.004</span> on the Boundless marketplace. Proven in <span className="font-mono">15s</span>.
      </>
    ),
    detail: "Outsourced to Boundless provers",
  },
  {
    num: "03",
    title: "Verify",
    description: (
      <>
        A Solidity contract checks the proof on-chain (<span className="font-mono">~250K</span> gas) and stores the price. Safety checks enforce minimum samples, deviation limits, and block freshness. Your protocol calls latestRoundData() -- zero code changes.
      </>
    ),
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

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4 sm:gap-5">
          {steps.map((step, i) => (
            <Reveal key={step.num} delay={i * 100}>
              <div className="bg-white border border-border rounded-xl p-6 h-full flex flex-col hover:border-border-strong hover:-translate-y-0.5 transition-all duration-200">
                <span className="font-mono text-accent text-sm font-bold mb-3">
                  {step.num}
                </span>
                <h3 className="font-display text-xl font-bold text-text-primary mb-3">
                  {step.title}
                </h3>
                <p className="text-sm text-text-secondary leading-relaxed font-body mb-4 flex-1">
                  {step.description}
                </p>
                <span className="font-mono bg-bg-surface rounded-full px-3 py-1 text-xs text-accent font-semibold self-start">
                  {step.detail}
                </span>
              </div>
            </Reveal>
          ))}
        </div>

        {/* Security callout */}
        <Reveal>
          <div className="mt-10 sm:mt-14 bg-accent/5 border border-accent/20 rounded-xl p-6 sm:p-7">
            <div className="flex items-start gap-3">
              <span className="inline-flex items-center justify-center w-8 h-8 rounded-full bg-accent/10 text-accent shrink-0 mt-0.5">
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
                  Multi-block median. Can&apos;t flash-loan that.
                </h3>
                <p className="text-text-secondary font-body text-sm leading-relaxed mb-3">
                  We prove prices across a{" "}
                  <span className="text-accent font-semibold">
                    contiguous range of <span className="font-mono">10+</span> blocks
                  </span>{" "}
                  and take the median. The range is configurable — use more blocks for stronger guarantees. Flash loans execute within a single block and cannot influence the median across a range.
                </p>
                <p className="text-text-secondary font-body text-sm leading-relaxed">
                  To manipulate this feed, an attacker would need to sustain
                  manipulation across <span className="font-mono">&gt;50%</span> of sampled blocks while risking
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
