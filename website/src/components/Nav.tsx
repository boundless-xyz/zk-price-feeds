"use client";

import { useState, useEffect } from "react";

export default function Nav() {
  const [open, setOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <nav
      className={`fixed top-0 left-0 right-0 z-50 transition-colors duration-200 ${
        scrolled
          ? "bg-bg/95 backdrop-blur-sm border-b border-border"
          : "bg-transparent"
      }`}
    >
      <div className="bg-accent text-white text-center py-2 px-4 text-sm font-body">
        Built to be agent friendly. Deploy a feed programmatically with the{" "}
        <a
          href="https://github.com/boundless-xyz/zk-oracle"
          target="_blank"
          rel="noopener noreferrer"
          className="underline font-semibold hover:text-white/80"
        >
          CLI wizard
        </a>
        .
      </div>
      <div className="max-w-5xl mx-auto px-5 sm:px-8 py-3.5 flex items-center justify-between">
        <a
          href="#"
          className="font-display text-lg font-bold tracking-tight text-text-primary"
        >
          AnyFeed
        </a>

        {/* Desktop */}
        <div className="hidden sm:flex items-center gap-7 text-sm font-body">
          <a
            href="#search"
            className="text-text-secondary hover:text-text-primary transition-colors"
          >
            Search
          </a>
          <a
            href="#how-it-works"
            className="text-text-secondary hover:text-text-primary transition-colors"
          >
            How It Works
          </a>
          <a
            href="#cost"
            className="text-text-secondary hover:text-text-primary transition-colors"
          >
            Cost
          </a>
          <a
            href="#faq"
            className="text-text-secondary hover:text-text-primary transition-colors"
          >
            FAQ
          </a>
          <a
            href="https://github.com/boundless-xyz/boundless/discussions"
            target="_blank"
            rel="noopener noreferrer"
            className="text-text-secondary hover:text-text-primary transition-colors"
          >
            Help
          </a>
          <a
            href="https://github.com/boundless-xyz/zk-oracle"
            target="_blank"
            rel="noopener noreferrer"
            className="px-4 py-1.5 rounded-md bg-accent text-white font-medium text-sm transition-colors hover:bg-accent-hover"
          >
            GitHub
          </a>
        </div>

        {/* Mobile hamburger */}
        <button
          onClick={() => setOpen(!open)}
          className="sm:hidden flex flex-col gap-1.5 p-2 -mr-2"
          aria-label="Toggle menu"
          aria-expanded={open}
        >
          <span
            className={`block w-5 h-0.5 bg-text-primary transition-transform duration-200 ${
              open ? "rotate-45 translate-y-2" : ""
            }`}
          />
          <span
            className={`block w-5 h-0.5 bg-text-primary transition-opacity duration-200 ${
              open ? "opacity-0" : ""
            }`}
          />
          <span
            className={`block w-5 h-0.5 bg-text-primary transition-transform duration-200 ${
              open ? "-rotate-45 -translate-y-2" : ""
            }`}
          />
        </button>
      </div>

      {/* Mobile dropdown */}
      <div
        className={`sm:hidden border-t border-border bg-bg overflow-hidden transition-all duration-200 ${
          open ? "max-h-64 opacity-100" : "max-h-0 opacity-0"
        }`}
      >
        <div className="px-5 py-5 flex flex-col gap-4 text-sm font-body">
          <a
            href="#search"
            onClick={() => setOpen(false)}
            className="text-text-secondary hover:text-text-primary"
          >
            Search
          </a>
          <a
            href="#how-it-works"
            onClick={() => setOpen(false)}
            className="text-text-secondary hover:text-text-primary"
          >
            How It Works
          </a>
          <a
            href="#cost"
            onClick={() => setOpen(false)}
            className="text-text-secondary hover:text-text-primary"
          >
            Cost
          </a>
          <a
            href="#faq"
            onClick={() => setOpen(false)}
            className="text-text-secondary hover:text-text-primary"
          >
            FAQ
          </a>
          <a
            href="https://github.com/boundless-xyz/boundless/discussions"
            target="_blank"
            rel="noopener noreferrer"
            onClick={() => setOpen(false)}
            className="text-text-secondary hover:text-text-primary"
          >
            Help
          </a>
          <a
            href="https://github.com/boundless-xyz/zk-oracle"
            target="_blank"
            rel="noopener noreferrer"
            onClick={() => setOpen(false)}
            className="px-4 py-2.5 rounded-md bg-accent text-white font-medium text-sm text-center"
          >
            GitHub
          </a>
        </div>
      </div>
    </nav>
  );
}
