"use client";

import { useState, useCallback, useEffect, useRef } from "react";
import PriceTicker from "./PriceTicker";

interface DexPair {
  chainId: string;
  dexId: string;
  pairAddress: string;
  baseToken: { address: string; name: string; symbol: string };
  quoteToken: { address: string; name: string; symbol: string };
  priceUsd: string;
  liquidity: { usd: number };
  volume: { h24: number };
  fdv: number;
}

const SUPPORTED_DEXES: Record<string, string> = {
  uniswap: "Uniswap V2/V3",
  pancakeswap: "PancakeSwap",
  sushiswap: "SushiSwap",
  curve: "Curve",
  balancer: "Balancer V2",
  aerodrome: "Aerodrome",
  camelot: "Camelot",
  trader_joe: "Trader Joe",
  traderjoe: "Trader Joe",
  dodo: "DODO",
  velodrome: "Velodrome",
  baseswap: "BaseSwap",
  quickswap: "QuickSwap",
  spookyswap: "SpookySwap",
};

const EVM_CHAINS: Record<string, string> = {
  ethereum: "Ethereum",
  bsc: "BSC",
  polygon: "Polygon",
  arbitrum: "Arbitrum",
  optimism: "Optimism",
  base: "Base",
  avalanche: "Avalanche",
  fantom: "Fantom",
  linea: "Linea",
  scroll: "Scroll",
  blast: "Blast",
  mantle: "Mantle",
  zksync: "zkSync",
  celo: "Celo",
  gnosis: "Gnosis",
  moonbeam: "Moonbeam",
  cronos: "Cronos",
};

const EXAMPLE_FEEDS = [
  { token: "ETH", address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", dex: "Uniswap V3", chain: "Ethereum", hasChainlink: true },
  { token: "UNI", address: "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984", dex: "Uniswap V2", chain: "Ethereum", hasChainlink: true },
  { token: "LINK", address: "0x514910771AF9Ca656af840dff83E8264EcF986CA", dex: "Balancer V2", chain: "Ethereum", hasChainlink: true },
  { token: "AERO", address: "0x940181a94A35A4569E4529A3CDfB74e38FD98631", dex: "Aerodrome", chain: "Base", hasChainlink: false },
  { token: "RPL", address: "0xD33526068D116cE69F19A9ee46F0bd304F21A51f", dex: "Uniswap V2", chain: "Ethereum", hasChainlink: false },
  { token: "CAKE", address: "0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82", dex: "PancakeSwap", chain: "BSC", hasChainlink: false },
];

function isEvmChain(chainId: string): boolean {
  return chainId in EVM_CHAINS;
}

function getDexLabel(dexId: string): string {
  const lower = dexId.toLowerCase();
  for (const [key, label] of Object.entries(SUPPORTED_DEXES)) {
    if (lower.includes(key)) return label;
  }
  return dexId;
}

function formatUsd(value: number | undefined): string {
  if (!value) return "$0";
  if (value >= 1_000_000) return `$${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `$${(value / 1_000).toFixed(0)}K`;
  return `$${value.toFixed(0)}`;
}

function formatPrice(price: string | undefined): string {
  if (!price) return "N/A";
  const n = parseFloat(price);
  if (n >= 1) return `$${n.toFixed(2)}`;
  if (n >= 0.01) return `$${n.toFixed(4)}`;
  if (n >= 0.0001) return `$${n.toFixed(6)}`;
  return `$${n.toPrecision(3)}`;
}

export default function Hero() {
  const [query, setQuery] = useState("");
  const [pairs, setPairs] = useState<DexPair[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [searched, setSearched] = useState(false);
  const [selectedPair, setSelectedPair] = useState<DexPair | null>(null);
  const [copied, setCopied] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  const doSearch = useCallback(async (input: string) => {
    const trimmed = input.trim();
    if (!trimmed) return;

    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;

    setLoading(true);
    setError("");
    setPairs([]);
    setSearched(true);
    setSelectedPair(null);

    const url = new URL(window.location.href);
    url.searchParams.set("token", trimmed);
    window.history.replaceState({}, "", url.toString());

    try {
      const isAddress = /^0x[a-fA-F0-9]{40}$/.test(trimmed);
      let allPairs: DexPair[] = [];

      if (isAddress) {
        // Direct token address lookup -- gets all pools
        const res = await fetch(
          `https://api.dexscreener.com/latest/dex/tokens/${encodeURIComponent(trimmed)}`,
          { signal: controller.signal },
        );
        if (!res.ok) throw new Error(`DexScreener returned ${res.status}`);
        const data = await res.json();
        allPairs = data.pairs || [];
      } else {
        // Search by name/symbol -- returns limited results
        const searchRes = await fetch(
          `https://api.dexscreener.com/latest/dex/search?q=${encodeURIComponent(trimmed)}`,
          { signal: controller.signal },
        );
        if (!searchRes.ok) throw new Error(`DexScreener returned ${searchRes.status}`);
        const searchData = await searchRes.json();
        const searchPairs: DexPair[] = searchData.pairs || [];

        if (searchPairs.length > 0) {
          // Filter to EVM chains first so non-EVM results don't waste token slots
          const evmSearchPairs = searchPairs.filter((p) => isEvmChain(p.chainId));
          // Collect unique token addresses from search results to fetch ALL their pools
          const seen = new Set<string>();
          const tokenAddresses: string[] = [];
          for (const p of evmSearchPairs) {
            const addr = p.baseToken.address.toLowerCase();
            if (!seen.has(addr)) {
              seen.add(addr);
              tokenAddresses.push(p.baseToken.address);
            }
            if (tokenAddresses.length >= 5) break; // Top 5 matching tokens
          }

          // Fetch all pools for each token in parallel
          const fetches = tokenAddresses.map(async (addr) => {
            const res = await fetch(
              `https://api.dexscreener.com/latest/dex/tokens/${addr}`,
              { signal: controller.signal },
            );
            if (!res.ok) return [];
            const data = await res.json();
            return (data.pairs || []) as DexPair[];
          });

          const results = await Promise.all(fetches);
          // Merge and deduplicate by pairAddress+chainId
          const pairMap = new Map<string, DexPair>();
          for (const pairList of results) {
            for (const p of pairList) {
              pairMap.set(`${p.pairAddress}-${p.chainId}`, p);
            }
          }
          allPairs = Array.from(pairMap.values());
        }
      }

      const evmPairs = allPairs
        .filter((p: DexPair) => isEvmChain(p.chainId))
        .sort(
          (a: DexPair, b: DexPair) =>
            (b.liquidity?.usd || 0) - (a.liquidity?.usd || 0),
        );
      setPairs(evmPairs);
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") return;
      setError(
        "Couldn't find pools. Try a token name (e.g. \"AERO\") or a contract address (0x...).",
      );
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const token = params.get("token");
    if (token) {
      setQuery(token);
      doSearch(token);
    }
  }, [doSearch]);

  function clearSearch() {
    abortRef.current?.abort();
    setSearched(false);
    setPairs([]);
    setSelectedPair(null);
    setError("");
    setQuery("");
    const url = new URL(window.location.href);
    url.searchParams.delete("token");
    window.history.replaceState({}, "", url.pathname);
  }

  function copyCliCommand(pair: DexPair) {
    const cmd = `git clone https://github.com/boundless-xyz/zk-oracle && cd zk-oracle && POOL_ADDRESS=${pair.pairAddress} cargo run --release --bin wizard`;
    navigator.clipboard.writeText(cmd);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  const showingExamples = !searched && !loading;

  return (
    <section id="search" className="pt-36 sm:pt-44 pb-16 sm:pb-24 relative overflow-hidden">
      <PriceTicker />
      <div className="max-w-5xl mx-auto px-5 sm:px-8 relative">
        {/* Headline — centered */}
        <div className="text-center mb-12">
          <h1 className="text-5xl sm:text-6xl lg:text-7xl font-display font-bold leading-[1.1] tracking-tight text-text-primary mb-5">
            A price feed for{" "}
            <br className="hidden sm:block" />
            every token.
          </h1>
          <p className="text-lg text-text-secondary max-w-2xl mx-auto leading-relaxed font-body">
            ZK-verified DEX prices for any on-chain pair. No subscription. No
            vendor contract. If you have a DEX pool address you have a price feed.
          </p>
        </div>

        {/* Search — centered, larger, more prominent */}
        <div className="max-w-2xl mx-auto mb-10">
          <div className="flex flex-col sm:flex-row gap-3">
            <input
              id="token-search"
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) =>
                e.key === "Enter" &&
                (searched ? clearSearch() : doSearch(query))
              }
              placeholder="Token name or address (e.g. AERO, ETH, 0x...)"
              className="flex-1 bg-white border-2 border-border-strong rounded-xl px-5 py-4 text-lg text-text-primary font-mono placeholder:text-text-muted/50 focus:outline-none focus:ring-2 focus:ring-accent/30 focus:border-accent transition-all"
            />
            <button
              onClick={searched ? clearSearch : () => doSearch(query)}
              disabled={loading}
              className={`rounded-xl px-6 py-4 font-semibold text-base transition-colors disabled:opacity-50 whitespace-nowrap ${
                searched
                  ? "bg-bg-surface border-2 border-border-strong text-text-primary hover:bg-bg-raised"
                  : "bg-accent text-white hover:bg-accent-hover"
              }`}
            >
              {loading ? "Searching..." : searched ? "Clear" : "Look up pools"}
            </button>
          </div>
          <p className="text-sm text-text-muted font-body mt-3 text-center">
            {searched
              ? "Data via DexScreener — sorted by liquidity"
              : "Search by name, symbol, or address. Works on Ethereum, Base, Arbitrum, and 14 more chains."}
          </p>
        </div>

        {/* Selected pair cost card */}
        {selectedPair && (
          <div className="max-w-2xl mx-auto mb-10 bg-white border border-border rounded-xl p-6 relative animate-fade-in">
            <button
              onClick={() => setSelectedPair(null)}
              className="absolute top-4 right-5 text-text-muted hover:text-text-primary text-lg leading-none"
              aria-label="Close"
            >
              &times;
            </button>

            <div className="flex flex-col sm:flex-row sm:items-start gap-4 mb-6">
              <div className="flex-1">
                <div className="flex items-center gap-2 mb-1.5">
                  <span className="text-xl font-bold text-text-primary font-display">
                    {selectedPair.baseToken.symbol}
                  </span>
                  <span className="text-text-muted">/</span>
                  <span className="text-text-secondary font-body">
                    {selectedPair.quoteToken.symbol}
                  </span>
                </div>
                <div className="flex flex-wrap gap-1.5">
                  <span className="text-xs font-mono text-text-muted bg-bg-surface px-2 py-0.5 rounded">
                    {EVM_CHAINS[selectedPair.chainId] || selectedPair.chainId}
                  </span>
                  <span className="text-xs font-mono text-text-muted bg-bg-surface px-2 py-0.5 rounded">
                    {getDexLabel(selectedPair.dexId)}
                  </span>
                </div>
              </div>
              <div className="sm:text-right">
                <div className="text-3xl font-mono font-bold text-text-primary">
                  {formatPrice(selectedPair.priceUsd)}
                </div>
                <div className="text-xs text-text-muted mt-0.5">
                  current price
                </div>
              </div>
            </div>

            {/* Cost breakdown — three items in a row with + signs */}
            <div className="flex items-center justify-center gap-3 sm:gap-4 mb-6">
              <div className="text-center">
                <div className="text-lg sm:text-xl font-mono font-bold text-text-primary">
                  $0.004
                </div>
                <div className="text-[10px] text-text-muted mt-0.5">
                  ZK proving
                </div>
              </div>
              <span className="text-xl font-mono text-text-muted">+</span>
              <div className="text-center">
                <div className="text-lg sm:text-xl font-mono font-bold text-text-primary">
                  $0.007
                </div>
                <div className="text-[10px] text-text-muted mt-0.5">
                  L2 gas
                </div>
              </div>
              <span className="text-xl font-mono text-text-muted">=</span>
              <div className="text-center">
                <div className="text-xl sm:text-2xl font-mono font-bold text-accent">
                  $0.01
                </div>
                <div className="text-[10px] text-text-muted mt-0.5">
                  total / update
                </div>
              </div>
            </div>

            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
              <div className="flex gap-4 text-xs font-mono text-text-muted">
                <span>
                  Liq:{" "}
                  <span className="text-text-secondary">
                    {formatUsd(selectedPair.liquidity?.usd)}
                  </span>
                </span>
                <span>
                  24h:{" "}
                  <span className="text-text-secondary">
                    {formatUsd(selectedPair.volume?.h24)}
                  </span>
                </span>
              </div>
              <div className="flex items-center gap-3">
                <a
                  href="https://github.com/boundless-xyz/zk-oracle#quick-start"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="px-4 py-2.5 rounded-lg bg-accent text-white font-semibold text-sm transition-colors hover:bg-accent-hover text-center"
                >
                  Deploy oracle
                </a>
                <button
                  onClick={() => copyCliCommand(selectedPair)}
                  className="px-4 py-2.5 rounded-lg border border-border-strong text-text-primary text-sm font-mono transition-colors hover:bg-bg-surface"
                >
                  {copied ? "Copied!" : "Copy CLI command"}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Error */}
        {error && (
          <div className="max-w-2xl mx-auto mb-6">
            <p className="text-negative font-mono text-sm">{error}</p>
          </div>
        )}

        {/* No results */}
        {searched && !loading && pairs.length === 0 && !error && (
          <div className="max-w-2xl mx-auto mb-6">
            <p className="text-text-secondary font-body">
              No DEX pools found on EVM chains. This token might only trade on
              non-EVM chains or centralized exchanges.
            </p>
          </div>
        )}

        {/* Loading */}
        {loading && (
          <div className="max-w-2xl mx-auto mb-6">
            <p className="text-text-muted font-mono text-sm">
              Searching DEX pools...
            </p>
          </div>
        )}

        {/* Search results — clean table, no card wrapper */}
        {pairs.length > 0 && !selectedPair && (
          <div className="max-w-2xl mx-auto mb-10 animate-fade-in">
            <div className="flex items-center justify-between mb-3">
              <span className="text-xs font-mono text-text-muted">
                {pairs.length} pool{pairs.length !== 1 && "s"} found
              </span>
              <span className="text-xs text-accent font-semibold font-body hidden sm:block">
                Click a pool to see cost
              </span>
              <span className="text-xs text-accent font-semibold font-body sm:hidden">
                Tap for cost
              </span>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left py-2.5 px-3 text-[11px] font-medium text-text-muted">
                      Pair
                    </th>
                    <th className="text-left py-2.5 px-3 text-[11px] font-medium text-text-muted hidden sm:table-cell">
                      DEX
                    </th>
                    <th className="text-left py-2.5 px-3 text-[11px] font-medium text-text-muted hidden md:table-cell">
                      Chain
                    </th>
                    <th className="text-right py-2.5 px-3 text-[11px] font-medium text-text-muted">
                      Price
                    </th>
                    <th className="text-right py-2.5 px-3 text-[11px] font-medium text-text-muted hidden sm:table-cell">
                      Liquidity
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {pairs.slice(0, 12).map((pair, idx) => (
                    <tr
                      key={pair.pairAddress + pair.chainId}
                      onClick={() => setSelectedPair(pair)}
                      className={`border-b border-border/50 hover:bg-bg-surface transition-colors cursor-pointer ${
                        idx === 0 ? "border-l-2 border-l-accent" : ""
                      }`}
                    >
                      <td className="py-3 px-3">
                        <span className="font-semibold text-text-primary font-body text-sm">
                          {pair.baseToken.symbol}
                        </span>
                        <span className="text-text-muted text-xs">
                          {" "}/ {pair.quoteToken.symbol}
                        </span>
                        {idx === 0 ? (
                          <span className="block text-[10px] text-accent font-medium mt-0.5">
                            Highest liquidity
                          </span>
                        ) : (
                          <span className="block sm:hidden text-[10px] text-text-muted mt-0.5">
                            {getDexLabel(pair.dexId)} &middot;{" "}
                            {EVM_CHAINS[pair.chainId] || pair.chainId}
                          </span>
                        )}
                      </td>
                      <td className="py-3 px-3 text-sm text-text-secondary font-body hidden sm:table-cell">
                        {getDexLabel(pair.dexId)}
                      </td>
                      <td className="py-3 px-3 hidden md:table-cell">
                        <span className="text-sm text-text-secondary">
                          {EVM_CHAINS[pair.chainId] || pair.chainId}
                        </span>
                      </td>
                      <td className="py-3 px-3 text-right font-mono text-text-primary text-sm">
                        {formatPrice(pair.priceUsd)}
                      </td>
                      <td className="py-3 px-3 text-right font-mono text-text-secondary text-sm hidden sm:table-cell">
                        {formatUsd(pair.liquidity?.usd)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {pairs.length > 12 && (
              <div className="text-center py-3 text-text-muted text-xs border-t border-border/50">
                + {pairs.length - 12} more pools
              </div>
            )}
          </div>
        )}

        {/* Back to list */}
        {pairs.length > 0 && selectedPair && (
          <div className="max-w-2xl mx-auto mb-6">
            <button
              onClick={() => setSelectedPair(null)}
              className="text-accent hover:text-accent-hover text-sm font-body font-medium transition-colors"
            >
              &larr; View all {pairs.length} pools
            </button>
          </div>
        )}

        {/* Example feeds (default state) — clean table, no card wrapper */}
        {showingExamples && (
          <div className="max-w-2xl mx-auto mb-14">
            <p className="text-xs font-body text-text-muted mb-3 text-center">
              Or try these tokens:
            </p>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left py-2.5 px-3 text-[11px] font-medium text-text-muted">
                      Token
                    </th>
                    <th className="text-left py-2.5 px-3 text-[11px] font-medium text-text-muted hidden sm:table-cell">
                      DEX
                    </th>
                    <th className="text-right py-2.5 px-3 text-[11px] font-medium text-text-muted">
                      Cost
                    </th>
                    <th className="text-right py-2.5 px-3 text-[11px] font-medium text-text-muted">
                      Chainlink?
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {EXAMPLE_FEEDS.map((feed) => (
                    <tr
                      key={feed.token}
                      onClick={() => {
                        setQuery(feed.address);
                        doSearch(feed.address);
                      }}
                      className="border-b border-border/50 hover:bg-bg-surface transition-colors cursor-pointer"
                    >
                      <td className="py-3 px-3">
                        <span className="font-semibold text-text-primary font-body">
                          {feed.token}
                        </span>
                        <span className="block sm:hidden text-[10px] text-text-muted mt-0.5">
                          {feed.dex} &middot; {feed.chain}
                        </span>
                      </td>
                      <td className="py-3 px-3 text-sm text-text-secondary font-body hidden sm:table-cell">
                        {feed.dex}
                        <span className="text-text-muted text-xs ml-2">
                          {feed.chain}
                        </span>
                      </td>
                      <td className="py-3 px-3 text-right font-mono text-accent font-bold text-sm">
                        $0.01
                      </td>
                      <td className="py-3 px-3 text-right">
                        {feed.hasChainlink ? (
                          <span className="bg-accent-dim text-accent text-xs rounded-full px-2 py-0.5 font-medium">
                            Has feed
                          </span>
                        ) : (
                          <span className="bg-negative-dim text-negative text-xs rounded-full px-2 py-0.5 font-semibold">
                            No feed
                          </span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Key metrics row — 5 items centered, no card wrapper */}
        <div className="flex flex-wrap justify-center gap-x-10 sm:gap-x-14 lg:gap-x-16 gap-y-6">
          <div className="text-center">
            <div className="text-3xl sm:text-4xl font-mono font-bold text-text-primary tracking-tight">
              500K+
            </div>
            <div className="text-sm text-text-muted font-body mt-1">
              on-chain pairs
            </div>
          </div>
          <div className="text-center">
            <div className="text-3xl sm:text-4xl font-mono font-bold text-accent tracking-tight">
              $0.01
            </div>
            <div className="text-sm text-text-muted font-body mt-1">
              per update on L2
            </div>
          </div>
          <div className="text-center">
            <div className="text-3xl sm:text-4xl font-mono font-bold text-text-primary tracking-tight">
              15s
            </div>
            <div className="text-sm text-text-muted font-body mt-1">
              proving time
            </div>
          </div>
          <div className="text-center">
            <div className="text-3xl sm:text-4xl font-mono font-bold text-text-primary tracking-tight">
              8
            </div>
            <div className="text-sm text-text-muted font-body mt-1">
              DEX types
            </div>
          </div>
          <div className="text-center">
            <div className="text-3xl sm:text-4xl font-mono font-bold text-text-primary tracking-tight">
              <span className="inline-flex items-center gap-1.5">
                <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-accent">
                  <polyline points="4 17 10 11 4 5" />
                  <line x1="12" y1="19" x2="20" y2="19" />
                </svg>
              </span>
            </div>
            <div className="text-sm text-text-muted font-body mt-1">
              agent-ready
            </div>
          </div>
        </div>

        {/* Agent-friendly callout */}
        <div className="mt-8 flex justify-center">
          <a
            href="https://github.com/boundless-xyz/zk-price-feeds"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 bg-accent/5 border border-accent/15 rounded-full px-5 py-2 text-sm text-text-secondary hover:border-accent/30 hover:bg-accent/8 transition-colors duration-200"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-accent">
              <polyline points="4 17 10 11 4 5" />
              <line x1="12" y1="19" x2="20" y2="19" />
            </svg>
            <span className="font-body">AI agents can deploy feeds programmatically.</span>
            <span className="font-mono text-accent text-xs">llms.txt</span>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="text-text-muted">
              <path d="M7 17L17 7M17 7H7M17 7V17" />
            </svg>
          </a>
        </div>
      </div>
    </section>
  );
}
