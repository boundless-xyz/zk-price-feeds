const steps = [
  {
    num: "01",
    title: "Read",
    gradient: "from-neon-cyan to-neon-blue",
    hoverGlow: "hover-glow-cyan",
    description:
      "A program inside the zkVM reads DEX pool state across 20 blocks. Supports 10 DEX standards: Uniswap V2/V3, Curve, Balancer, PancakeSwap, Aerodrome, Trader Joe, DODO, and more. Any EVM chain. Every read is cryptographically tied to actual on-chain state.",
  },
  {
    num: "02",
    title: "Prove",
    gradient: "from-neon-purple to-neon-magenta",
    hoverGlow: "hover-glow-purple",
    description:
      "Boundless computes the median price and generates a ZK proof. 73M cycles. $0.004 proving + gas. Done in 15 seconds.",
  },
  {
    num: "03",
    title: "Verify",
    gradient: "from-neon-green to-neon-cyan",
    hoverGlow: "hover-glow-green",
    description:
      "A Solidity contract verifies the proof on-chain (~250K gas) and stores the price. Your protocol calls latestRoundData() -- the same interface as Chainlink. Zero code changes.",
  },
];

export default function HowItWorks() {
  return (
    <section id="how-it-works" className="py-28 px-6 relative overflow-hidden">
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-neon-purple/5 rounded-full blur-[200px]" />

      <div className="max-w-6xl mx-auto relative z-10">
        <div className="text-center mb-16">
          <h2 className="text-4xl sm:text-6xl font-display tracking-wide text-text-primary">
            Three steps. That&apos;s it.
          </h2>
        </div>

        <div className="grid md:grid-cols-3 gap-6">
          {steps.map((step) => (
            <div
              key={step.num}
              className={`relative glass-strong rounded-2xl p-8 ${step.hoverGlow} transition-all duration-500 group hover:-translate-y-2`}
            >
              <div className="absolute top-4 right-6 text-7xl font-mono font-black text-white/[0.03]">
                {step.num}
              </div>
              <div className={`h-1 w-16 rounded-full bg-gradient-to-r ${step.gradient} mb-6`} />
              <h3 className="text-3xl font-display tracking-wide mb-4">
                <span className={`bg-gradient-to-r ${step.gradient} bg-clip-text text-transparent`}>
                  {step.title}
                </span>
              </h3>
              <p className="text-text-secondary leading-relaxed font-body">
                {step.description}
              </p>
            </div>
          ))}
        </div>

        {/* Security callout */}
        <div className="mt-12 glass-strong rounded-2xl p-8 max-w-3xl mx-auto text-center">
          <h3 className="text-2xl font-display tracking-wide gradient-text-pink-purple mb-4">
            20 blocks. Can&apos;t flash-loan that.
          </h3>
          <p className="text-text-secondary font-body">
            A flash loan manipulates price within one transaction. Our oracle computes the
            median across <span className="text-neon-cyan font-bold">20 blocks (~4 minutes)</span>.
            An attacker would need to control reserves across 11+ consecutive blocks -- costing
            tens of millions in capital at risk. Configurable from 5 to 100 blocks.
          </p>
        </div>
      </div>
    </section>
  );
}
