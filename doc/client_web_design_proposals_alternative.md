# client_web — Alternative Design Proposals: Neon Arcade Future

A second aesthetic direction: bold, playful, unapologetically modern. Where the first proposal channels an 18th-century gaming salon, this one asks: *what if trictrac ran on a holographic table in a Tokyo arcade, 2089?*

This document proposes a complete visual redesign with no obligation to mirror physical game objects. The priority is delight, readability, and memorability.

---

## Aesthetic Direction: "Holographic Arcade"

**Core concept**: The board floats in dark space as a self-illuminated slab. Fields pulse with neon light. Checkers are luminous marbles that leave light trails as they move. Scoring events trigger particle explosions. Every interaction has a micro-animation.

**The one unforgettable thing**: When a hole is won, the entire board floods with a colour wave — a full-screen shimmer that fades in 800ms — like a pinball machine tilting into multiball.

**Color palette**: Built on darkness, with high-saturation accents — cyan, magenta, gold. Not gradients on white (the generic AI aesthetic); instead, near-black backgrounds with glowing, luminous elements.

**Typography**: [Space Grotesk](https://fonts.google.com/specimen/Space+Grotesk) is overused. Instead:
- Display: [Syne](https://fonts.google.com/specimen/Syne) — geometric, confident, slightly alien
- Numerics: [DM Mono](https://fonts.google.com/specimen/DM+Mono) — for scores, dice values, field numbers — crisp monospace with personality
- UI labels: [Outfit](https://fonts.google.com/specimen/Outfit) — friendly, modern, clear at small sizes

```css
:root {
  /* Base */
  --void:           #09090f;        /* near-black with blue tint */
  --surface:        #12121f;        /* board slab */
  --surface-raised: #1a1a2e;        /* panels, cards */
  --surface-glass:  rgba(255,255,255,0.05); /* glassmorphism */

  /* Neon accents */
  --cyan:           #00e5ff;
  --cyan-dim:       #0099bb;
  --magenta:        #e040fb;
  --gold:           #ffd740;
  --gold-dim:       #c8a820;
  --green-neon:     #69ff47;
  --orange-neon:    #ff6d3a;

  /* Player colors */
  --player-white:   #e8e0ff;        /* soft violet-white */
  --player-black:   #1a0040;        /* deep indigo-black */
  --player-white-glow: #b39ddb;
  --player-black-glow: #7c4dff;

  /* Typography */
  --font-display:   'Syne', sans-serif;
  --font-mono:      'DM Mono', monospace;
  --font-ui:        'Outfit', sans-serif;

  /* Glow radii */
  --glow-sm:  0 0 8px;
  --glow-md:  0 0 16px;
  --glow-lg:  0 0 32px;
}
```

---

## 1. Board: A Floating Holographic Slab

**Concept**: The board is a dark rectangular surface that appears to float — slight perspective tilt (CSS `perspective` + `rotateX(3deg)`), a thin neon border (1px cyan on top, 1px dimmer on bottom for depth), and a subtle inner glow that makes the board feel luminous from within.

```css
.board {
  background: var(--surface);
  border: 1px solid var(--cyan-dim);
  box-shadow:
    0 0 0 1px rgba(0,229,255,0.1),
    0 0 40px rgba(0,229,255,0.08),
    0 24px 60px rgba(0,0,0,0.8);
  transform: perspective(1200px) rotateX(2deg);
  transform-origin: center bottom;
  border-radius: 4px;
}
```

The board background gets a very subtle **noise texture overlay** (SVG `<feTurbulence>` or a PNG grain layer at 3% opacity) — just enough to prevent it from looking like a flat rectangle, giving it material presence.

The center bar and side bars become **glowing dividers**: 4px wide, gradient from `var(--cyan)` at top to `var(--magenta)` at bottom, with a matching glow.

---

## 2. Fields: Neon Triangles with Zone Color Identity

Triangular fields (CSS `clip-path: polygon`) are essential here — they're geometric and modern, not just historically authentic.

Each quarter gets its **own neon color identity**, using a very dark base with a glowing triangle border:

| Quarter | Fields | Primary accent | Secondary (alternating) |
|---------|--------|---------------|------------------------|
| Small jan | 1–6 | `#00e5ff` (cyan) | `#0077aa` (dim cyan) |
| Big jan | 7–12 | `#7c4dff` (violet) | `#4a2a99` (dim violet) |
| Return jan | 13–18 | `#e040fb` (magenta) | `#991a99` (dim magenta) |
| Last jan | 19–24 | `#ffd740` (gold) | `#aa8800` (dim gold) |

The field itself is dark (`#14141f`). The color lives in a **glowing triangle border** — achieved with a layered `clip-path` + `::before` pseudo-element 2px larger that shows through as the border, with a CSS `filter: blur(3px)` outer glow:

```css
.field {
  background: #14141f;
  clip-path: polygon(50% 0%, 0% 100%, 100% 100%);
  position: relative;
}
.field::before {
  content: '';
  position: absolute;
  inset: -2px;
  background: var(--field-accent-color);
  clip-path: polygon(50% 0%, 0% 100%, 100% 100%);
  filter: blur(4px);
  opacity: 0.4;
  z-index: -1;
}
```

**On hover (clickable fields)**: the glow intensifies (`opacity: 0.9`, `filter: blur(6px)`) and the field interior lightens slightly. A ripple animation radiates outward from the click point.

**Selected field**: the entire field interior fills with a semi-transparent neon color — not just the border — and a 2px dashed animated border spins around it (`animation: spin-border 1s linear infinite`).

---

## 3. Checkers: Luminous Marbles

Forget CSS circles with radial gradients. Each checker is a **glowing orb** with:

- A dark, slightly translucent core
- A radial highlight in the upper-left (simulating a point light source)
- A colored halo that radiates outward onto the field triangle
- A subtle inner reflection ring

```css
.checker.white {
  background: radial-gradient(circle at 35% 30%,
    #ffffff,
    #c8c0e0 40%,
    #8878c0 70%,
    #3a2a60
  );
  box-shadow:
    inset 0 2px 6px rgba(255,255,255,0.8),
    inset 0 -2px 4px rgba(0,0,0,0.4),
    0 0 12px rgba(179,157,219,0.6),   /* violet-white glow */
    0 0 24px rgba(124,77,255,0.3);    /* outer violet halo */
}

.checker.black {
  background: radial-gradient(circle at 35% 30%,
    #7c4dff,
    #4a2d99 40%,
    #1a0a40 70%,
    #09040f
  );
  box-shadow:
    inset 0 2px 6px rgba(124,77,255,0.5),
    inset 0 -2px 4px rgba(0,0,0,0.8),
    0 0 12px rgba(124,77,255,0.7),
    0 0 24px rgba(124,77,255,0.3);
}
```

**Stack depth**: A stack of N checkers renders with each checker offset by 6px vertically and slightly scaled (0.97× per level deeper), creating genuine 3D stack depth without any 3D CSS transform. The count label floats above as a monospace number in `var(--gold)`.

**Selection animation**: On click to select, the top checker of the stack does a quick `scale(1.2) translateY(-8px)` bounce (150ms spring easing), then settles at `scale(1.1) translateY(-4px)` while selected.

**Movement animation**: When a move is confirmed (board state diff), selected checkers do a **light-trail arc** — a bezier path from origin field center to destination, with a fading cyan streak left behind (`box-shadow` animated along the path via `@property` interpolation or JS Web Animation API). Duration: 300ms.

---

## 4. Dice: Holographic Crystals

Replace the SVG ivory dice with **translucent crystal cubes**:

- Each die face is a dark glass square with a thin neon border
- Pips are glowing dots — cyan for normal, gold for doubles
- The die face has a subtle `backdrop-filter: blur(4px)` on a glass background

```css
.die-face rect {
  fill: rgba(255, 255, 255, 0.04);
  stroke: var(--cyan);
  stroke-width: 1.5;
  rx: 6;
  filter: drop-shadow(0 0 6px var(--cyan));
}
.die-face circle {
  fill: var(--cyan);
  filter: drop-shadow(0 0 4px var(--cyan));
}
```

**Double dice**: Both pips and borders switch to `var(--gold)`, with a stronger glow (`drop-shadow(0 0 8px var(--gold))`).

**Roll animation**: 600ms sequence —
1. Both dice **shatter outward** (`scale(0) rotate(720deg)`, opacity 0 → 1) appearing from nothing
2. During 400ms they rapidly cycle through face values (random pips swap every 60ms via CSS `animation`)
3. Final 200ms they decelerate and **snap** to the rolled values with a brief flash pulse

**Used die**: Fades the border to `rgba(255,255,255,0.1)` and dims pips to `rgba(255,255,255,0.2)` — the die goes "offline." A thin strikethrough line appears diagonally.

---

## 5. The Hole Tracker: Orbital Rings

Instead of progress bars, score and hole progress are visualised as **concentric orbital rings** beside each player's name panel — inspired by loading spinners, but static and data-driven.

- **Outer ring** (thick, 6px): hole progress. 12 segments, each one lights up as a hole is won. Segments are `var(--gold)` when won, near-invisible dark when empty.
- **Inner ring** (thin, 3px): point progress within the current hole. Continuously filled arc from 0° to (points/12 × 360°). Color: `var(--cyan)` for the active player, `var(--magenta)` for the opponent.

The arc fills animate with `stroke-dashoffset` transition (0.4s ease-out) on every point gain.

**Bredouille state**: The outer ring segments pulse — a slow `opacity: 0.6 → 1 → 0.6` sinusoidal glow — as long as bredouille is active. A small flag icon (⚑) in `var(--gold)` appears beside the ring.

---

## 6. Scoring Events: Light Shows

### Hole won — Full-board colour wave
A `position:fixed` `::after` overlay expands from the scoring player's side of the board:
- Radial gradient expanding from one edge: `rgba(255,215,64,0)` → `rgba(255,215,64,0.15)` → `rgba(255,215,64,0)`
- Duration: 800ms, ease-in-out
- Simultaneously: the scoring player's orbital rings segments animate sequentially (each segment snaps on with a 50ms delay)
- A large centered text `"+1 TROU"` in `var(--font-display)` at 3rem scales from 60% to 110% with `opacity: 0 → 1 → 0`, duration 1.2s

### Bredouille — The cascade
On top of the hole wave, add:
- A **confetti burst** of small colored squares (pure CSS: 20 `<span>` elements with randomised `animation-delay` and `translate`/`rotate` keyframes) in cyan, magenta, gold
- The `"+1 TROU"` text instead reads `"BREDOUILLE ×2"` in `var(--magenta)`
- The board border flashes: `border-color` cycles cyan → magenta → gold → cyan over 0.6s

### Jan scored — Notification card
Each jan scored gets a **toast card** that slides in from the right edge:
- Dark glass background (`rgba(26,26,46,0.95)`) with a left border in the jan's quarter color
- Jan name in `var(--font-ui)` bold, points in `var(--font-mono)` large
- Progress: `"+4 pts"` in cyan, `"+6 pts (double)"` in gold
- Cards stack vertically if multiple jans fire; each staggered by 80ms
- Auto-dismiss with a rightward slide-out after 3s

### Hit scored — Ripple on the target checker
When a hit is scored on a specific field, that field's checker emits a **sonar ripple**:
- 3 concentric rings expand from the checker's center, each `opacity: 1 → 0, scale: 1 → 2.5`
- Color: cyan for true hits, magenta for false hits (giving to opponent)
- Duration: 600ms per ring, staggered by 200ms

---

## 7. Player Panels: Glassmorphism Cards

Replace the cream `background: #f5edd8` panels with **glass cards** floating above the void:

```css
.player-score-panel {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-top: 1px solid rgba(255, 255, 255, 0.2); /* top catches light */
  backdrop-filter: blur(12px) saturate(1.5);
  border-radius: 12px;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.08);
}
```

**Active player panel**: the border glow of the active player's card brightens: `border-color: var(--cyan)` with `box-shadow: 0 0 16px rgba(0,229,255,0.2)`. A tiny animated pulse on the left edge (`width: 3px, animation: pulse 1.5s ease-in-out infinite`) indicates it is their turn.

**Player name**: Displayed in `var(--font-display)` at 1.1rem. A small colored dot (cyan for player 1, magenta for player 2) precedes the name — acts as the "you" indicator without needing a text suffix.

---

## 8. Status Bar: Dynamic Ambient Messaging

Replace the single plain-text status line with a **contextual bar** that changes character per game stage:

| Stage | Style | Color |
|-------|-------|-------|
| Waiting for opponent | Slow pulsing dots animation `... ` | Dim white |
| Your turn (roll) | "YOUR MOVE" in `var(--font-display)` with a blinking cursor | Cyan |
| Opponent's turn | Subtle shimmer on text | Dim magenta |
| Move selection | "SELECT CHECKER ①" with animated underline on "SELECT" | Cyan |
| Hold or Go | "SCORE!" with a spinning star ✦ | Gold |
| Paused (continue) | The bar has a pulsing amber background strip | Amber |
| Game over | Text cycles through all player colors | Full rainbow |

The bar itself is 3px tall and spans the full board width, showing a **neon progress shimmer** during the opponent's turn (a traveling gleam, like CSS `animation: shimmer` on a gradient).

---

## 9. Jan Zone Awareness: Neon Underlay

Rather than labels, the four quarters glow with their zone color in the background of the board — very subtle, just 4% opacity fills under the triangles:

```css
.board-quarter-small-jan  { background: rgba(0, 229, 255, 0.04); }
.board-quarter-big-jan    { background: rgba(124, 77, 255, 0.04); }
.board-quarter-return-jan { background: rgba(224, 64, 251, 0.04); }
.board-quarter-last-jan   { background: rgba(255, 215, 64, 0.04); }
```

When hovering a scoring-notification row that references a specific jan (e.g. "Big jan conserved"), the corresponding quarter's background pulses from 4% → 15% opacity for 600ms. This replaces the arrow overlay with a spatial, zone-level highlight — more legible and visually coherent.

---

## 10. Rest Corner: The Crown Field

Field 12 (White) and 13 (Black) get a distinct appearance:

- The triangle is outlined in `var(--gold)` instead of its quarter's color
- A small **crown SVG** (⚜ or ♛) floats centered in the triangle at 30% opacity when empty, brighter when held
- When the player holds the corner (2 checkers there), the triangle interior fills with a very subtle gold shimmer animation (`background-position: 0% → 100%` on a diagonal gradient, 2s loop)
- When the corner is available to be taken *par puissance*, the crown pulses at 1Hz

---

## 11. Login Screen: Warp Speed Entrance

**Hero**: A dark void with an animated **particle field** — small white/cyan dots drifting slowly, like stars. Pure CSS with 50 `<span>` elements (or a single `<canvas>` for performance), each with randomised `animation-delay` and drift keyframe.

**Title**: "TRICTRAC" in `var(--font-display)` at 5rem, with a **chromatic aberration effect** — three slightly offset copies in cyan, magenta, and white, blended with `mix-blend-mode: screen`. The word appears with a `clip-path: inset(100% 0 0 0) → inset(0% 0 0 0)` reveal animation (the text "rises" into view).

**Tagline**: `"XVIIIe siècle · En ligne · ∞"` in `var(--font-mono)` at 0.85rem, appearing letter-by-letter with a 20ms interval.

**Mode cards**: Instead of three buttons, three **holographic tiles** in a row:
- Each is a glass card with an icon, label, and a colored accent strip on the bottom
- On hover: the card lifts (`translateY(-4px)`) and the bottom strip color floods the card (low opacity fill)
- CREATE: cyan accent; JOIN: violet accent; vs BOT: orange accent

**Room code input**: Dark glass input with a cyan border glow on focus, monospace font for the code, no placeholder text (just a blinking cursor showing it's ready). The input border animates a traveling gleam on focus.

---

## 12. Game-Over Screen: Score Reveal Ceremony

Instead of a modal over a frozen game, the game-over sequence is a **full-page takeover**:

1. **Board fades out** (800ms fade): the board dims to 20% opacity
2. **Score card rises** from the bottom: a tall glass card with both players' hole counts displayed large in `var(--font-display)` — `"8"` vs `"3"` — in their respective colors
3. **Winner highlight**: the winning number scales up to 200% with a gold burst radiation behind it
4. **Bredouille annotation**: if applicable, `"× 2"` appears beside the number with a magenta glow, then the number updates to the effective doubled count
5. **Continue options**: two buttons slide up last — "QUIT" and "REJOUER" — with the rejouer button pulsing in cyan

---

## 13. Global Micro-Interactions

These apply throughout and give the interface a consistently tactile feel:

- **Button press**: `scale(0.96)` on `:active`, 80ms, then spring back. No `opacity` change — scale is more physical.
- **Button focus**: neon outline ring animated in from 0 to full radius (not the browser default outline).
- **Panel hover**: glass cards shift `box-shadow` slightly for a lifted feel.
- **Page load**: all elements stagger in with a `translateY(10px) → 0 + opacity 0 → 1`, each component with a `animation-delay` offset (board: 0ms, panels: 100ms, side panel: 200ms).
- **Custom cursor** (optional): replace the default cursor with a small circle that trails slightly behind the real cursor position — creates a luxurious "lag" feeling. Pure JS: interpolate cursor position toward mouse position at 80% each frame.

---

## Implementation Notes for Leptos/WASM

### What's straightforward in pure CSS
- All color variables, glass panels, glow effects, orbital rings (SVG `stroke-dashoffset`)
- Dice roll animation (CSS keyframes)
- Toast slide-ins (CSS `@keyframes` + `animation`)
- Confetti (CSS `@keyframes` on positioned `<div>` elements)
- Particle field on login (CSS-only with many `<span>` elements)

### What needs a small JS/WASM component
- **Board perspective tilt** with mouse-tracking (subtle parallax) — `mouse_position` signal driving CSS custom property
- **Checker light-trail movement** — needs previous/next board diff, then Web Animation API or `requestAnimationFrame`
- **Chromatic aberration on title** — CSS filter or SVG filter, but the animation needs JS timing

### What needs Rust/Leptos state
- **Board diff for animation**: store previous `[i8; 24]` alongside current in `GameScreen` as a `Memo`, compute moved checkers
- **Event timing for sequences**: hole-won wave → score reveal → dismiss must be orchestrated; a `RwSignal<Option<AnimationState>>` in `GameScreen` drives each phase

### Progressive approach
The proposals above can be adopted incrementally. Suggested order:
1. CSS variables + dark theme + Syne/DM Mono fonts → immediate impact, zero logic change
2. Glass panels, neon borders, glow effects → pure CSS
3. Orbital ring score tracker → SVG component
4. Triangular fields + zone colors → `board.rs` structural change
5. Dice animation → CSS keyframes in `die.rs`
6. Toast notifications → new `toast.rs` component
7. Hole-won wave → CSS overlay + Leptos signal
8. Checker animation → board diff + Web Animation API
