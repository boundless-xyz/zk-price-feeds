"use client";

import { useState } from "react";

export default function Footer() {
  const [copied, setCopied] = useState(false);

  const command =
    "git clone https://github.com/boundless-xyz/zk-oracle && cd zk-oracle && POOL_ADDRESS=0x... RPC_URL=https://... cargo run --release";

  function copyCommand() {
    navigator.clipboard.writeText(command);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <footer className="py-20 sm:py-28 px-5 sm:px-8 bg-code-bg text-code-text">
      <div className="max-w-5xl mx-auto">
        <h2 className="text-3xl sm:text-4xl font-display font-bold tracking-tight mb-8">
          Deploy a feed in 5 minutes.
        </h2>

        {/* Code block with copy */}
        <div className="bg-code-surface rounded-lg overflow-hidden max-w-lg mb-10 relative group">
          <div className="flex items-center gap-2 px-4 py-2.5 border-b border-white/5">
            <div className="flex gap-1.5" aria-hidden="true">
              <div className="w-2.5 h-2.5 rounded-full bg-code-muted/40" />
              <div className="w-2.5 h-2.5 rounded-full bg-code-muted/40" />
              <div className="w-2.5 h-2.5 rounded-full bg-code-muted/40" />
            </div>
            <span className="text-code-muted text-xs font-mono ml-2">
              terminal
            </span>
            <button
              onClick={copyCommand}
              className="ml-auto text-code-muted hover:text-code-text text-xs font-mono transition-colors opacity-0 group-hover:opacity-100"
              aria-label="Copy command"
            >
              {copied ? "Copied!" : "Copy"}
            </button>
          </div>
          <pre
            className="p-4 sm:p-6 overflow-x-auto text-xs sm:text-sm leading-relaxed font-mono cursor-pointer"
            onClick={copyCommand}
          >
            <code>
              <span className="text-code-accent">$</span>
              <span className="text-code-text">
                {" "}
                git clone https://github.com/boundless-xyz/zk-oracle
              </span>
              {"\n"}
              <span className="text-code-accent">$</span>
              <span className="text-code-text"> cd zk-oracle</span>
              {"\n\n"}
              <span className="text-code-muted">
                # Set your pool address and run
              </span>
              {"\n"}
              <span className="text-code-accent">$</span>
              <span className="text-code-text">
                {" "}
                POOL_ADDRESS=0x... RPC_URL=https://... cargo run --release
              </span>
            </code>
          </pre>
        </div>

        {/* Links */}
        <div className="flex items-center gap-3 flex-wrap mb-12">
          <a
            href="https://github.com/boundless-xyz/zk-oracle"
            target="_blank"
            rel="noopener noreferrer"
            className="px-5 py-2.5 rounded-md bg-accent text-white font-semibold text-sm transition-colors hover:bg-accent-hover"
          >
            GitHub
          </a>
          <a
            href="https://docs.beboundless.xyz"
            target="_blank"
            rel="noopener noreferrer"
            className="px-5 py-2.5 rounded-md border border-white/15 text-code-text font-medium text-sm hover:bg-white/5 transition-colors"
          >
            Docs
          </a>
          <a
            href="https://beboundless.xyz"
            target="_blank"
            rel="noopener noreferrer"
            className="px-5 py-2.5 rounded-md border border-white/15 text-code-text font-medium text-sm hover:bg-white/5 transition-colors"
          >
            Boundless
          </a>
        </div>

        <a
          href="https://beboundless.xyz"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-2 text-code-muted hover:text-code-text transition-colors text-sm font-body mb-6"
        >
          Powered by
          <span className="font-display font-bold text-base text-code-text/80">Boundless</span>
        </a>

        <p className="text-code-muted text-sm font-body">
          Built with{" "}
          <a
            href="https://github.com/risc0/risc0"
            target="_blank"
            rel="noopener noreferrer"
            className="text-code-text/70 hover:text-code-text transition-colors"
          >
            Steel
          </a>{" "}
          +{" "}
          <a
            href="https://risczero.com"
            target="_blank"
            rel="noopener noreferrer"
            className="text-code-text/70 hover:text-code-text transition-colors"
          >
            RISC Zero
          </a>{" "}
          +{" "}
          <a
            href="https://beboundless.xyz"
            target="_blank"
            rel="noopener noreferrer"
            className="text-code-text/70 hover:text-code-text transition-colors"
          >
            Boundless
          </a>
        </p>
      </div>
    </footer>
  );
}
