# client_web — UI/UX Design Proposals

A structured critique of the current interface compared to the physical game, followed by concrete upgrade proposals. Organised from most impactful to most effort-intensive.

---

## Aesthetic Direction

**Concept: "18th-century French gaming salon"**

The physical trictrac board is a piece of furniture — carved mahogany rails, felt or baize surface, ivory and ebony checkers, brass pegs in drilled holes, gilt scoring tokens. The online interface should feel like playing on that table under candlelight in a Parisian salon.

- **Typography**: Pair a classical serif (e.g. [Cormorant Garamond](https://fonts.google.com/specimen/Cormorant+Garamond)) for headings and score readouts with a clean humanist sans (e.g. [Jost](https://fonts.google.com/specimen/Jost)) for UI controls and status text.
- **Palette**: Forest green felt board (`#1d3d28`), alternating ivory (`#f0e6c8`) and deep burgundy (`#7a1e2a`) triangular fields, dark mahogany rails (`#2a1508`), aged parchment panels (`#f2e8d0`), gilt gold accents (`#c8a448`).
- **Unforgettable detail**: Triangular fields (true *flèches*) rendered in CSS with a wood-grain body surrounding them.

---

## 1. Board Shape: Rectangles → True Triangles

**Current state**: Fields are 60×180px rectangles with a rounded corner. No backgammon/trictrac board looks like this.

**Physical game**: Fields are elongated triangles (*flèches*) pointing from the rail toward the center bar, alternating two colors.

**Proposal**: Replace `.field` `<div>` elements with SVG triangles, or use CSS `clip-path: polygon(50% 0%, 0% 100%, 100% 100%)` for bottom-row triangles and `clip-path: polygon(0% 0%, 100% 0%, 50% 100%)` for top-row triangles. The field background and checker stack become SVG foreignObject or positioned elements inside. This is a large structural change to `board.rs` but is the single highest-impact visual improvement.

The board body between triangles becomes visible as the wood/felt surface — this naturally creates the physical board's "relief" without any extra decoration.

---

## 2. Jan Zone Visual Identity

**Current state**: The four quarters (small jan, big jan, return jan, last jan) are visually identical — same field color scheme, no labels or separation beyond the center bar.

**Physical game**: Players must constantly know which quarter they are in because the rules differ radically per zone (forbidden jans, filling values, hit scoring values differ between small-jan-table and big-jan-table, corner position, exit zone).

**Proposals**:

### 2a. Zone labels
Add thin labels (`"petit jan"`, `"grand jan"`, `"jan de retour"`, `"dernier jan"`) beneath the board-row (or as a subtle strip above/below the quarter). These should use the serif font at very small size and low opacity — decorative, not noisy.

### 2b. Field color shift per zone
The physical game uses alternating colors within each quarter, but different quarters can use slightly different base hues:
- Small jan (fields 1–6): warm ivory / burgundy
- Big jan / corner zone (fields 7–12): same, but field 12 gets a distinct "corner" treatment (see §4)
- Return jan (fields 13–18): very subtly cooler ivory / dark teal instead of burgundy — signals "opponent's territory"
- Last jan / exit (fields 19–24): subtly warmer, indicating checkers are "almost home"

### 2c. Small-jan-table / big-jan-table highlight during hit scoring
When a hit is being scored, briefly tint the entire table (fields 1–12 or 13–24) to make the point value distinction (4 pts vs 2 pts) spatially obvious. This fires as a 300ms flash synchronized with the scoring notification.

---

## 3. Rest Corner (Field 12 / 13) Special Appearance

**Current state**: Field 12 looks identical to field 11. Nothing indicates its unique rules (must enter/leave with 2 checkers simultaneously, cannot be landed on by a single checker).

**Physical game**: The corner is a corner — it is literally in the corner of the table, a distinct physical location.

**Proposals**:
- Give field 12 (and 13 for Black) a **crown or arch shape** at the tip of the triangle, using a small SVG ornament.
- Apply a **slightly warmer gold** field color to distinguish it.
- When the player has two checkers there, show a subtle **lock icon** or a gilded ring around the checker stack to indicate "corner held."
- When the corner is available to be taken *par puissance*, add a gentle pulsing outline on field 12 to indicate the privilege is available.
- Tooltip or popover: on hover, show a brief note "Coin de repos — must enter and leave with 2 checkers."

---

## 4. Checker Rendering: Static → Animated

**Current state**: Checkers appear and disappear between `ViewState` snapshots. No movement animation.

**Physical game**: Checkers slide across the board with a satisfying click sound.

**Proposals**:

### 4a. Slide animation
Diff the board array between the previous and current `ViewState`. For each checker that moved from field A to field B, apply a CSS or Web Animation API translation from `field_center(A)` to `field_center(B)` (duration ~250ms, ease-out). This requires keeping the previous `ViewState` as state in `GameScreen` and computing a diff when a new state arrives.

### 4b. Lift effect during staging
When the player clicks an origin field and a checker becomes "selected," apply a `transform: scale(1.15) translateY(-4px)` with a subtle drop shadow increase. Visually lifts the checker off the board.

### 4c. Checker appearance
Replace the CSS `radial-gradient` circles with SVG:
- **White**: ivory `#f5edd8` with a pearl sheen gradient, thin gilt ring border, engraved concentric circles
- **Black**: ebony `#1a0f06` with subtle grain texture, same gilt ring

A stack of 5+ checkers can render a "perspective stack" — each checker at a slight y offset with a shadow, giving depth.

---

## 5. Dice: Static → Rolling Animation

**Current state**: Dice appear with their final value immediately. No sense of randomness or anticipation.

**Physical game**: Dice are shaken in a cup (*cornet*) and tumbled out. The roll is a theatrical moment.

**Proposals**:

### 5a. Roll animation
When `SerTurnStage` transitions from `RollDice` to `Move`, animate both dice with a fast face-cycling (showing random faces for ~400ms, decelerating to final value). Pure CSS `animation` on the die-face SVG circles, cycling via `keyframes`.

### 5b. Dice cups
Add two SVG/CSS dice cups above the dice display. During rolling, they visually "tip" (rotate 90° via CSS transform) and the dice "fall out." A subtle translate-y on the dice moves them downward into view.

### 5c. Double visual
When both dice show the same value, add a subtle golden glow around both — visually communicating that it is a double (which affects scoring: 6 pts instead of 4, etc.).

### 5d. Used-die visual
When one die has been consumed by a staged move, slide it slightly down and reduce opacity (current: gray-out). Animate the "used" transition with `transition: all 0.15s`.

---

## 6. Scoring Notifications: Side Panel → Layered Toasts

**Current state**: Scoring events appear as a small cream panel in the side panel column (`scoring-panel`). They are easily missed, especially opponent events.

**Physical game**: Scoring is the central drama of every turn — points are loudly marked, bredouille doubled, holes recorded with pegs.

**Proposals**:

### 6a. Board-overlaid toast for holes
When a hole is won, display a large centered overlay on the board — not a modal, but a translucent toast with gilt border: `"Trou ! ×2 bredouille"`. Auto-dismiss after 1.5s or on click. This is the most important event and deserves the most visual weight.

### 6b. Scoring event animation
When `my_scored_event` appears, animate the panel sliding in from the right with a 200ms ease-out. Jan rows stagger in (each with `animation-delay: n * 50ms`).

### 6c. Jan hover → board highlight synchronization
The current arrow-on-hover feature is good. Extend it: when hovering a jan row, also highlight the relevant fields with a faint golden shimmer instead of (or in addition to) the arrows. This ties the abstract jan name to a concrete board location.

### 6d. Bredouille treatment
Bredouille doubles a hole's value — a massive game event. Currently shown as a small amber badge. Proposals:
- The toast for a bredouille hole should animate in differently: bigger, gold shimmer background
- Show a small animated flag (*pavillon*) icon in the score panel when bredouille is active, matching the physical game's token

### 6e. Hit scoring visual
When a hit is scored (*battue*), show a brief visual on the opponent's half-field checker — a faint concentric ring expanding outward (CSS `animation: ripple 0.4s ease-out`). This communicates the "fictitious" nature of the hit: something happened at that checker's position, but it didn't move.

---

## 7. Score Panel: Progress Bars → Pegs and Holes

**Current state**: Points and holes are displayed as progress bars (0–12) and numeric values. Functional but abstract.

**Physical game**: Points are tracked with physical tokens (*jetons*) placed on the board surface at specific field tips. Holes are tracked with pegs (*fichets*) in holes drilled along the rail at each field base.

**Proposals**:

### 7a. Hole tracker: 12 dots/pegs
Replace the `score-bar-holes` progress bar with a row of 12 small circles ("drilled holes") in a horizontal strip. Filled holes are rendered as a gilded peg inserted (solid gold circle). Unfilled holes are empty rings. This is a `<svg>` with 12 `<circle>` elements. The filled count animates one peg at a time (sequenced `animation-delay`).

### 7b. Point tracker: token on board
For points (0–11), show a small token image positioned at the corresponding field tip along the near rail — mirroring the physical game exactly. This is ambitious but highly authentic. A simpler approach: replace the thin progress bar with a 12-cell dot track where one glowing token is positioned.

### 7c. Bredouille indicator
When `can_bredouille` is true for a player, show the token as a double-token (two stacked icons) or add a small flag icon next to the token.

---

## 8. Status Communication: Text → Contextual Guidance

**Current state**: A single text line (`"Select move 1"`, `"Opponent's turn"`) in the status bar. New players have no idea what to do or why they can't do something.

**Physical game**: Human players narrate what's happening; experienced players understand the state from context.

**Proposals**:

### 8a. Contextual sub-prompt
Below the primary status, show a secondary hint line in smaller text:
- During `Move` stage: `"Click a highlighted field to move a checker"`
- During `HoldOrGoChoice`: `"Hold to keep points and keep playing — Go to reset and start a new setting"`
- When waiting for confirm: `"↑ Opponent scored points — click Continue when ready"`

### 8b. Forbidden-jan visual cue
When a field is in the opponent's jan and the player cannot land there (forbidden jan rule), show those fields with a subtle `✕` pattern or darker tint rather than just being unclickable. This communicates *why* the fields aren't selectable.

### 8c. Exit-eligible highlight
When all player checkers are in the last jan (fields 19–24), add a subtle directional glow to the exit rail (the right/left edge of the board depending on player). A small "EXIT →" arrow indicator could appear.

### 8d. Can-take-corner indicator
When the player can take their corner (field 12 or 13 is the valid destination), add a brief pulse to that field beyond the standard `.dest` highlight — the corner rules are special enough to warrant extra visual salience.

---

## 9. Bug Fix: Hold Button Is Non-Functional

**File**: `src/components/scoring.rs` line 91

The "Hold" button in the `ScoringPanel` has no `on:click` handler. In the physical game, "Hold" (*tenir*) means: stay in the current setting, mark remainder points, and continue playing normally.

`PlayerAction` does not currently include a `Hold` variant. In the current implementation, if the player simply does nothing (doesn't click Go), the game waits — but there is no message sent to the backend to confirm "staying."

**Fix required**: Add `PlayerAction::Hold` (or reuse `Mark`) and connect the Hold button's `on:click` to send it. The backend needs to handle it by advancing past `HoldOrGoChoice` without triggering `GameEvent::Go`.

---

## 10. Layout: Side Panel → Integrated Design

**Current state**: The board and side panel sit side-by-side (`board-and-panel: flex-direction row`). The side panel (min-width 160px) contains status, dice, scoring, and buttons stacked vertically.

**Proposals**:

### 10a. Move dice inside the board
Place the dice display centered in the **board-bar** (the vertical divider between quarters). Currently the bar is 20px wide — widen it to ~80px and center two dice there. This puts dice physically near the board action, matching the physical game where dice land on the board surface. The bar color becomes a darker felt strip.

### 10b. Status bar above the board
Move the primary status message to a full-width strip directly above the board, styled with the serif font at larger size. This gives it appropriate visual weight and removes it from the cramped side panel.

### 10c. Action buttons below the board (or in score panels)
"Continue," "Go," and "Hold" buttons can live below the board in a centered button row. The side panel then becomes purely informational (scoring panels), which can slide in from the right.

### 10d. Mobile: rotate board 90° option
The board is ~776px wide. On narrow screens, offer a portrait mode where the board is rendered rotated 90° (each player's quarters stacked vertically), with a scroll-independent panel above/below for controls.

---

## 11. Login Screen: Form → Atmosphere

**Current state**: A plain 320px-wide column with a `<h1>Trictrac</h1>`, a text input, and three buttons. Functional but gives no sense of what the game is.

**Physical game**: A trictrac board is an object of beauty — players set it out, prepare the checkers, and roll for first-move privilege.

**Proposals**:

### 11a. Illustrated header
A high-quality SVG illustration of the board (simplified top-down view, showing the triangular fields, checker stacks at starting positions, dice) as the page hero. Possibly animated: the two stacks slowly deploying two checkers as the page loads.

### 11b. Typography treatment
"TRICTRAC" as a large display heading in a classical-weight serif, possibly with subtle tracking and a gilt color. Below it, the French subtitle: *"Jeu de trictrac — XVIIIe siècle"* in small-caps at reduced opacity.

### 11c. Mode selection
The three buttons (Create / Join / vs Bot) styled as wooden tiles or embossed cards rather than plain buttons.

---

## 12. Game-Over Modal: Generic → Ceremonial

**Current state**: A centered modal with "Game Over," the winner's name, and Quit/Play Again buttons.

**Physical game**: The end of a game involves settling accounts, noting the final hole count, and potentially recording results.

**Proposals**:
- Show a **final score parchment** — both players' hole counts displayed like a ledger entry, with the winner's name engraved in gilt text
- Animate the modal entrance with a slight downward reveal (the parchment "unrolling")
- Show the hole difference: `"8 — 3"` in large numerals with a small flourish between them
- If bredouille applied to the winning holes: `"✕ 2 bredouille"` annotation
- "Play again" styled as "Rejouer" / "Play again" with a dice icon

---

## Implementation Priority

| Priority | Proposal | Effort | Impact |
|----------|-----------|--------|--------|
| 1 | §9 Fix Hold button (bug) | Low | Correctness |
| 2 | §3 Rest corner special appearance | Low | Clarity |
| 3 | §8b–d Forbidden jan + exit + corner cues | Medium | Clarity |
| 4 | §5a–d Dice roll animation | Medium | Delight |
| 5 | §6a–b Scoring toasts + animation | Medium | Drama |
| 6 | §7a Hole tracker (12 peg dots) | Low | Authenticity |
| 7 | §2a–b Jan zone labels + color shift | Low | Orientation |
| 8 | §4a Checker slide animation | High | Polish |
| 9 | §1 Triangular fields | High | Authenticity |
| 10 | §10a–b Dice in bar + status above board | Medium | Layout |
| 11 | §6e Hit ripple animation | Medium | Comprehension |
| 12 | §11 Login redesign | Medium | First impression |
| 13 | §12 Game-over modal | Low | Finish |
| 14 | §4c SVG checkers | Medium | Aesthetics |
| 15 | §7b–c Token tracker on rail | High | Authenticity |

---

## Typography and CSS Variables Proposal

Replace the anonymous `sans-serif` body font and introduce a CSS variable system:

```css
@import url('https://fonts.googleapis.com/css2?family=Cormorant+Garamond:wght@400;600&family=Jost:wght@300;400;500&display=swap');

:root {
  /* Board */
  --board-felt:       #1d3d28;
  --board-rail:       #2a1508;
  --field-ivory:      #f0e6c8;
  --field-burgundy:   #7a1e2a;
  --field-corner:     #c8a030;    /* rest corner accent */
  --field-exit-glow:  #e8c060;

  /* Checkers */
  --checker-white:    #f5edd8;
  --checker-black:    #1a0f06;
  --checker-ring:     #c8a448;    /* gilt border */

  /* UI */
  --ui-parchment:     #f2e8d0;
  --ui-parchment-dark: #e4d8b8;
  --ui-ink:           #2a1a08;
  --ui-gold:          #c8a448;
  --ui-gold-dark:     #8a6a28;
  --ui-green-accent:  #3a6b2a;
  --ui-red-accent:    #7a1e2a;

  /* Typography */
  --font-display:     'Cormorant Garamond', Georgia, serif;
  --font-ui:          'Jost', system-ui, sans-serif;
}
```
