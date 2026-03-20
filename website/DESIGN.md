# DESIGN.md — AnyFeed

> Extends [Boundless Base Theme](~/.claude/design-library/boundless-base.md) with warm-light editorial overrides.
> Product site for ZK-verified DEX price feeds.

## Project Identity

**Name**: AnyFeed
**Tagline**: A price feed for every token.
**Audience**: DeFi developers integrating price oracles into smart contracts. They know Chainlink, Pyth, RedStone. They care about cost, coverage, and trust model.
**Goal**: Developer searches for a token → sees it works → copies the CLI command or deploys

## Personality Layer

### Light Mode — Warm Editorial
This is NOT a typical crypto site. Warm, not cold. Editorial, not flashy. Bloomberg-meets-Stripe.

```css
--color-bg: #faf9f6;              /* warm off-white */
--color-bg-surface: #f0eeea;      /* card backgrounds */
--color-text-primary: #1c1917;    /* warm charcoal */
--color-text-secondary: #44403c;
--color-text-muted: #78716c;
--color-accent: #059669;          /* emerald — the one brand color */
--color-accent-hover: #047857;
--color-border: rgba(28, 25, 23, 0.1);
```

No dark mode. The warm palette IS the identity.

### Display Font
**Space Grotesk** (700 for headlines) — geometric but warm, technical without being cold.
- Where: Hero headline, section headings
- Not: Body, nav, metadata (use DM Sans)

### Body Font
**DM Sans** (400/500) — clean, readable, slightly rounded.

### Mono Font
**JetBrains Mono** — code blocks, prices, gas costs, addresses, CLI commands.

### Accent
Emerald (#059669) — used for:
- Primary CTA buttons
- Active states and links
- The "Mathematics" trust badge
- Code syntax highlighting (green tokens)
- Never as text color on body copy

### Motion Style
**Moderate** — this is a marketing site with an interactive demo.
- Page load: staggered fade-up on hero elements, 400-600ms
- Section reveals: fade-up on scroll intersection (Reveal.tsx pattern)
- Hover: card lift (-2px translateY), 150ms
- Search: instant results, no loading skeleton needed (fast API)
- Tabs: content crossfade, 200ms
- Smooth scroll via Lenis

### Unique Components

- **Token search with live results** — the hero IS the product demo. DexScreener integration showing real pools.
- **Proportional comparison bar** — 3px (Chainlink) vs full-width (AnyFeed) showing the coverage gap
- **Provider comparison table** — trust model column is the differentiator ("Mathematics" vs "Committee")
- **Migration code tabs** — Chainlink/Pyth/RedStone/Chronicle with before/after code
- **Cost frequency table** — L1 vs L2 costs at different update frequencies

## Section Plan (10 sections, max)

1. **Nav** — Fixed header, "AnyFeed" wordmark, 5 nav links + GitHub CTA
2. **Hero** — "A price feed for every token." + token search + example feeds + key metrics (500K+, $0.01, 15s, 8 DEXes)
3. **Problem** — "Chainlink covers ~350 tokens. There are 500,000+ pairs." + proportional bars + category tags + DEX grid
4. **How It Works** — Read → Prove → Verify (3-step flow) + TWAP security callout
5. **Cost Comparison** — Provider table + 3 stat callouts (500K+, $0.004, Math)
6. **Zero Migration** — Code tabs showing drop-in replacement for each provider
7. **L2 Economics** — Cost frequency table + latency comparison
8. **Beyond Basic** — "This is the simplest thing we can prove." + 6 future computation cards
9. **FAQ** — 8 Q&A accordion
10. **Footer** — Deploy command + links + Boundless branding

## Layout Variation (break monotony)

The current site is monotonous — every section is full-width, left-aligned, same rhythm. Fix with:

| Section | Layout | Why |
|---------|--------|-----|
| Hero | Centered, full-width | Commanding entry point |
| Problem | Asymmetric 40/60, bars on right | Visual contrast |
| How It Works | 3-column cards | Clear progression |
| Cost Comparison | Full-width table + right-aligned callouts | Data density |
| Migration | Centered code block with tab row | Focus on code |
| L2 Economics | Split — table left, latency cards right | Comparison |
| Beyond | 3-column grid with featured first card | Hierarchy |
| FAQ | Centered, max-w-3xl | Reading focus |

## Image Direction
- No images. The search demo IS the visual.
- Icons: inline SVGs only (shield, check, arrow)
- Code blocks are the visual anchors (dark bg on warm page = contrast)

## Copy Voice
- Headlines: Short, declarative. "A price feed for every token." Not questions, not cleverness.
- Body: Technical but accessible. "TWAP + Median. Can't flash-loan that."
- CTAs: Action verbs. "Deploy a feed", "Look up pools", "Copy CLI command"
- Data: Let numbers speak. "$0.01", "500K+", "15s", "250K gas"

## Hard Rules
All base theme "Never Do" rules, plus:
- No crypto-bro aesthetics (neon, dark gradients, token logos, "wagmi")
- No illustrations — code and data are the visuals
- Emerald is the only accent color
- Every section must have a distinct layout (no two consecutive sections with same structure)
- The search demo must remain interactive and fast
- Monospace for ALL numbers (prices, gas, costs, cycle counts)
