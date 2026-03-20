"use client";

import { useEffect, useRef, useState } from "react";
import Reveal from "./Reveal";

const categories = [
  "Micro-cap DeFi tokens",
  "LP tokens & vault shares",
  "New launches (day-one feeds)",
  "Community tokens",
  "Cross-chain bridged assets",
  "RWA tokens without CEX listings",
];

const dexes = [
  { name: "Uniswap V2/V3", note: "Multi-block median" },
  { name: "Curve", note: "StableSwap" },
  { name: "Balancer V2", note: "Weighted" },
  { name: "Solidly / Aerodrome", note: "Stable" },
  { name: "Trader Joe LB", note: "Active bin" },
  { name: "DODO PMM", note: "Mid price" },
  { name: "Algebra V3", note: "Camelot" },
  { name: "Any V2 fork", note: "SushiSwap, PancakeSwap, ..." },
];

function useCountUp(target: number, active: boolean, duration = 1200) {
  const [value, setValue] = useState(0);
  useEffect(() => {
    if (!active) return;
    const start = performance.now();
    function tick(now: number) {
      const elapsed = now - start;
      const progress = Math.min(elapsed / duration, 1);
      // ease-out-expo
      const eased = progress === 1 ? 1 : 1 - Math.pow(2, -10 * progress);
      setValue(Math.round(eased * target));
      if (progress < 1) requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
  }, [active, target, duration]);
  return value;
}

export default function Problem() {
  const barsRef = useRef<HTMLDivElement>(null);
  const [animate, setAnimate] = useState(false);

  useEffect(() => {
    const el = barsRef.current;
    if (!el) return;
    const prefersReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (prefersReduced) {
      setAnimate(true);
      return;
    }
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setAnimate(true);
          observer.disconnect();
        }
      },
      { threshold: 0.4 }
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const chainlinkCount = useCountUp(350, animate);
  const pairsCount = useCountUp(500000, animate);

  return (
    <section className="py-20 sm:py-28 border-t border-border">
      <div className="max-w-5xl mx-auto px-5 sm:px-8">
        {/* Asymmetric 40/60 layout */}
        <div className="grid grid-cols-1 lg:grid-cols-[2fr_3fr] gap-12 lg:gap-16 mb-16 sm:mb-20">
          {/* Left column — headline + body (40%) */}
          <Reveal>
            <div>
              <h2 className="text-4xl sm:text-5xl lg:text-[3.5rem] font-display font-bold leading-[1.1] tracking-tight text-text-primary mb-4">
                Chainlink covers ~350 tokens.
              </h2>
              <p className="text-2xl sm:text-3xl lg:text-4xl font-display font-bold text-text-muted tracking-tight mb-6">
                There are 500,000+ pairs.
              </p>
              <p className="text-base sm:text-lg text-text-secondary max-w-md font-body leading-relaxed">
                Want a feed for your token? Get in line. Or pay <span className="font-mono">$50–200K</span>/year to
                sponsor one. We built something different.
              </p>
            </div>
          </Reveal>

          {/* Right column — animated proportional bars (60%) */}
          <div ref={barsRef} className="flex flex-col justify-center">
            <div className="space-y-10">
              {/* Chainlink — tiny bar */}
              <div>
                <div className="flex items-baseline justify-between mb-2">
                  <span className="text-sm font-body text-text-muted">Chainlink feeds</span>
                  <span className="text-sm font-mono font-bold text-text-muted tabular-nums">
                    ~{chainlinkCount.toLocaleString()}
                  </span>
                </div>
                <div className="h-2 bg-bg-raised rounded-full overflow-hidden">
                  <div
                    className="h-full bg-text-muted/40 rounded-full"
                    style={{
                      width: animate ? "0.07%" : "0%",
                      minWidth: animate ? "4px" : "0px",
                      transition: "width 1.2s cubic-bezier(0.16, 1, 0.3, 1), min-width 1.2s cubic-bezier(0.16, 1, 0.3, 1)",
                    }}
                  />
                </div>
              </div>

              {/* AnyFeed — massive bar */}
              <div>
                <div className="flex items-baseline justify-between mb-2">
                  <span className="text-sm font-body text-text-secondary font-medium">On-chain trading pairs</span>
                  <span className="text-xl font-mono font-bold text-accent tabular-nums">
                    {pairsCount.toLocaleString()}+
                  </span>
                </div>
                <div className="h-4 bg-accent/10 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-accent rounded-full"
                    style={{
                      width: animate ? "100%" : "0%",
                      transition: "width 1.4s cubic-bezier(0.16, 1, 0.3, 1) 0.15s",
                    }}
                  />
                </div>
              </div>
            </div>

            {/* Ratio callout */}
            <p
              className="mt-6 text-sm font-mono text-text-muted text-right tabular-nums"
              style={{
                opacity: animate ? 1 : 0,
                transition: "opacity 0.5s ease 1.4s",
              }}
            >
              1,400× more coverage
            </p>
          </div>
        </div>

        {/* Token categories */}
        <Reveal>
          <p className="text-[11px] font-body font-medium text-text-muted mb-3 uppercase tracking-wider">
            Tokens Chainlink will never support
          </p>
          <div className="flex flex-wrap gap-2 mb-14">
            {categories.map((cat) => (
              <span
                key={cat}
                className="bg-bg-surface border border-border rounded-full px-4 py-1.5 text-sm text-text-secondary font-body"
              >
                {cat}
              </span>
            ))}
          </div>
        </Reveal>

        {/* Supported DEXes */}
        <Reveal>
          <p className="text-[11px] font-body font-medium text-text-muted mb-4 uppercase tracking-wider">
            8 DEX types supported
          </p>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            {dexes.map((d) => (
              <div
                key={d.name}
                className="bg-white border border-border rounded-xl px-4 py-3 hover:border-border-strong hover:-translate-y-0.5 transition-all duration-200"
              >
                <span className="text-text-primary font-medium font-body text-sm block mb-0.5">
                  {d.name}
                </span>
                <span className="text-text-muted text-xs font-mono">
                  {d.note}
                </span>
              </div>
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  );
}
