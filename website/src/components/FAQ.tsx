"use client";

import { useState } from "react";
import Reveal from "./Reveal";

const faqs = [
  {
    q: "Is this a production oracle?",
    a: "Not yet. This is a template and POC. You deploy it for your pair. We don't operate feeds -- you run your own keeper or let anyone call updatePrice().",
  },
  {
    q: "How fresh is the price?",
    a: "As fresh as the last keeper update. Proving takes ~15 seconds, then it's a normal on-chain transaction. If you run a keeper every 30 seconds, your price is never more than ~45 seconds old.",
  },
  {
    q: "What DEXes do you support?",
    a: "8 DEX types: Uniswap V2, Uniswap V3, Curve, Balancer V2, Solidly/Aerodrome, Trader Joe LB, DODO PMM, and Algebra V3 (Camelot/QuickSwap). Any V2 fork works out of the box.",
  },
  {
    q: "Is it flash-loan resistant?",
    a: "Yes. We prove prices across a contiguous range of 10+ blocks and take the median. Flash loans execute within a single block — they cannot influence the median across a range. The range is configurable (use more blocks for stronger guarantees). An attacker would need to sustain manipulation across a majority of the range, risking arbitrageurs correcting the price every block.",
  },
  {
    q: "What chains?",
    a: "Any EVM chain supported by Steel. Currently tested on Ethereum mainnet and Base. L2 support is straightforward -- and that's where the economics get really good ($0.01/update).",
  },
  {
    q: "Can I use this with my existing Chainlink integration?",
    a: "Yes. We implement AggregatorV3Interface. Change the feed address and everything else stays the same. Drop-in adapters also available for Pyth, RedStone, and Chronicle.",
  },
  {
    q: "Who runs the keeper?",
    a: "You do, or your protocol does, or an MEV searcher does (they need fresh prices for liquidations). It's permissionless -- anyone can call updatePrice().",
  },
  {
    q: "Why is the on-chain gas higher than Pyth or RedStone?",
    a: "Because we verify a ZK proof on-chain (~250K gas for Groth16). Pyth/RedStone verify signatures (~65-90K gas). The tradeoff: their security comes from trusting a committee. Ours comes from math.",
  },
];

export default function FAQ() {
  const [open, setOpen] = useState<number | null>(null);

  return (
    <section id="faq" className="py-20 sm:py-28 border-t border-border">
      <div className="max-w-3xl mx-auto px-5 sm:px-8">
        <Reveal>
          <h2 className="text-3xl sm:text-4xl font-display font-bold tracking-tight text-text-primary mb-10 sm:mb-14 text-center">
            FAQ
          </h2>
        </Reveal>

        <div>
          {faqs.map((faq, i) => (
            <Reveal key={i} delay={i * 40}>
              <div className="border-b border-border">
                <button
                  onClick={() => setOpen(open === i ? null : i)}
                  className="w-full flex items-start justify-between gap-4 py-5 text-left group"
                  aria-expanded={open === i}
                >
                  <span className="text-base font-display font-semibold text-text-primary group-hover:text-accent transition-colors">
                    {faq.q}
                  </span>
                  <span
                    className={`text-text-muted shrink-0 mt-1 transition-transform duration-200 ${
                      open === i ? "rotate-45" : ""
                    }`}
                  >
                    <svg
                      width="14"
                      height="14"
                      viewBox="0 0 14 14"
                      fill="none"
                    >
                      <path
                        d="M7 1v12M1 7h12"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        strokeLinecap="round"
                      />
                    </svg>
                  </span>
                </button>
                <div
                  className="faq-panel"
                  data-open={open === i ? "true" : "false"}
                >
                  <div>
                    <p className="text-sm text-text-secondary font-body leading-relaxed pb-5">
                      {faq.a}
                    </p>
                  </div>
                </div>
              </div>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  );
}
