"use client";

import { useEffect, useRef } from "react";

const TOKENS = [
  { symbol: "ETH", price: "$1,847.23", up: true },
  { symbol: "UNI", price: "$8.76", up: false },
  { symbol: "LINK", price: "$14.52", up: true },
  { symbol: "AERO", price: "$0.89", up: true },
  { symbol: "RPL", price: "$3.14", up: false },
  { symbol: "CAKE", price: "$2.47", up: true },
  { symbol: "CRV", price: "$0.42", up: false },
  { symbol: "BAL", price: "$4.20", up: true },
  { symbol: "DODO", price: "$0.128", up: false },
  { symbol: "SUSHI", price: "$0.91", up: true },
  { symbol: "WBTC", price: "$67,432", up: true },
  { symbol: "ARB", price: "$0.54", up: false },
  { symbol: "OP", price: "$1.23", up: true },
  { symbol: "GMX", price: "$34.19", up: false },
  { symbol: "PENDLE", price: "$5.67", up: true },
  { symbol: "SNX", price: "$2.10", up: false },
  { symbol: "MKR", price: "$1,200", up: true },
  { symbol: "COMP", price: "$45.20", up: true },
  { symbol: "YFI", price: "$8,901", up: false },
  { symbol: "AAVE", price: "$156.89", up: true },
  { symbol: "LDO", price: "$1.87", up: false },
  { symbol: "FXS", price: "$3.42", up: true },
  { symbol: "CVX", price: "$2.76", up: false },
  { symbol: "JOE", price: "$0.31", up: true },
];

interface Ticker {
  x: number;
  y: number;
  speed: number;
  symbol: string;
  price: string;
  up: boolean;
  opacity: number;
  size: number;
}

export default function PriceTicker() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const tickersRef = useRef<Ticker[]>([]);
  const animRef = useRef<number>(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const prefersReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (prefersReduced) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    function resize() {
      if (!canvas) return;
      canvas.width = canvas.offsetWidth * 2;
      canvas.height = canvas.offsetHeight * 2;
    }
    resize();
    window.addEventListener("resize", resize);

    // Initialize tickers
    const count = 24;
    tickersRef.current = Array.from({ length: count }, (_, i) => {
      const token = TOKENS[i % TOKENS.length];
      return {
        x: Math.random() * 90 + 2,
        y: Math.random() * 120 - 10,
        speed: 0.006 + Math.random() * 0.012,
        symbol: token.symbol,
        price: token.price,
        up: token.up,
        opacity: 0.05 + Math.random() * 0.06, // 5-11%
        size: 11 + Math.random() * 4, // 11-15px
      };
    });

    function draw() {
      if (!canvas || !ctx) return;
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      for (const t of tickersRef.current) {
        t.y -= t.speed;
        if (t.y < -5) {
          t.y = 105;
          t.x = Math.random() * 90 + 2;
          const newToken = TOKENS[Math.floor(Math.random() * TOKENS.length)];
          t.symbol = newToken.symbol;
          t.price = newToken.price;
          t.up = Math.random() > 0.5; // randomize direction on reset
        }

        const px = (t.x / 100) * canvas.width;
        const py = (t.y / 100) * canvas.height;

        // Fade at top and bottom edges
        const edgeFade = Math.min(t.y / 15, (100 - t.y) / 15, 1);
        const alpha = t.opacity * Math.max(edgeFade, 0);

        // Token symbol in neutral
        const fontSize = t.size * 2;
        ctx.font = `600 ${fontSize}px "JetBrains Mono", "SF Mono", monospace`;
        ctx.fillStyle = `rgba(28, 25, 23, ${alpha})`;
        ctx.fillText(t.symbol, px, py);

        // Price with subtle green/red tint
        const symbolWidth = ctx.measureText(t.symbol).width;
        ctx.font = `400 ${fontSize}px "JetBrains Mono", "SF Mono", monospace`;
        if (t.up) {
          ctx.fillStyle = `rgba(5, 150, 105, ${alpha * 0.8})`; // emerald
        } else {
          ctx.fillStyle = `rgba(220, 38, 38, ${alpha * 0.6})`; // red, slightly more subtle
        }
        ctx.fillText(` ${t.price}`, px + symbolWidth, py);
      }

      animRef.current = requestAnimationFrame(draw);
    }

    animRef.current = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(animRef.current);
      window.removeEventListener("resize", resize);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="absolute inset-0 w-full h-full pointer-events-none"
      style={{
        maskImage: "linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%)",
        WebkitMaskImage: "linear-gradient(to bottom, transparent 0%, black 15%, black 85%, transparent 100%)",
      }}
      aria-hidden="true"
    />
  );
}
