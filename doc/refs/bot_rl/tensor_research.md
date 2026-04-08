# Tensor research

## Current tensor anatomy

[0..23] board.positions[i]: i8 ∈ [-15,+15], positive=white, negative=black (combined!)
[24] active player color: 0 or 1
[25] turn_stage: 1–5
[26–27] dice values (raw 1–6)
[28–31] white: points, holes, can_bredouille, can_big_bredouille
[32–35] black: same
─────────────────────────────────
Total 36 floats

The C++ side (ObservationTensorShape() → {kStateEncodingSize}) treats this as a flat 1D vector, so OpenSpiel's
AlphaZero uses a fully-connected network.

### Fundamental problems with the current encoding

1. Colors mixed into a signed integer. A single value encodes both whose checker is there and how many. The network
   must learn from a value of -3 that (a) it's the opponent, (b) there are 3 of them, and (c) both facts interact with
   all the quarter-filling logic. Two separate, semantically clean channels would be much easier to learn from.

2. No normalization. Dice (1–6), counts (−15 to +15), booleans (0/1), points (0–12) coexist without scaling. Gradient
   flow during training is uneven.

3. Quarter fill status is completely absent. Filling a quarter is the dominant strategic goal in Trictrac — it
   triggers all scoring. The network has to discover from raw counts that six adjacent fields each having ≥2 checkers
   produces a score. Including this explicitly is the single highest-value addition.

4. Exit readiness is absent. Whether all own checkers are in the last quarter (fields 19–24) governs an entirely
   different mode of play. Knowing this explicitly avoids the network having to sum 18 entries and compare against 0.

5. dice_roll_count is missing. Used for "jan de 3 coups" (must fill the small jan within 3 dice rolls from the
   starting position). It's in the Player struct but not exported.

## Key Trictrac distinctions from backgammon that shape the encoding

| Concept                   | Backgammon             | Trictrac                                                  |
| ------------------------- | ---------------------- | --------------------------------------------------------- |
| Hitting a blot            | Removes checker to bar | Scores points, checker stays                              |
| 1-checker field           | Vulnerable (bar risk)  | Vulnerable (battage target) but not physically threatened |
| 2-checker field           | Safe "point"           | Minimum for quarter fill (critical threshold)             |
| 3-checker field           | Safe with spare        | Safe with spare                                           |
| Strategic goal early      | Block and prime        | Fill quarters (all 6 fields ≥ 2)                          |
| Both colors on a field    | Impossible             | Perfectly legal                                           |
| Rest corner (field 12/13) | Does not exist         | Special two-checker rules                                 |

The critical thresholds — 1, 2, 3 — align exactly with TD-Gammon's encoding rationale. Splitting them into binary
indicators directly teaches the network the phase transitions the game hinges on.

## Options

### Option A — Separated colors, TD-Gammon per-field encoding (flat 1D)

The minimum viable improvement.

For each of the 24 fields, encode own and opponent separately with 4 indicators each:

own_1[i]: 1.0 if exactly 1 own checker at field i (blot — battage target)
own_2[i]: 1.0 if exactly 2 own checkers (minimum for quarter fill)
own_3[i]: 1.0 if exactly 3 own checkers (stable with 1 spare)
own_x[i]: max(0, count − 3) (overflow)
opp_1[i]: same for opponent
…

Plus unchanged game-state fields (turn stage, dice, scores), replacing the current to_vec().

Size: 24 × 8 = 192 (board) + 2 (dice) + 1 (current player) + 1 (turn stage) + 8 (scores) = 204
Cost: Tensor is 5.7× larger. In practice the MCTS bottleneck is game tree expansion, not tensor fill; measured
overhead is negligible.
Benefit: Eliminates the color-mixing problem; the 1-checker vs. 2-checker distinction is now explicit. Learning from
scratch will be substantially faster and the converged policy quality better.

### Option B — Option A + Trictrac-specific derived features (flat 1D)

Recommended starting point.

Add on top of Option A:

// Quarter fill status — the single most important derived feature
quarter_filled_own[q] (q=0..3): 1.0 if own quarter q is fully filled (≥2 on all 6 fields)
quarter_filled_opp[q] (q=0..3): same for opponent
→ 8 values

// Exit readiness
can_exit_own: 1.0 if all own checkers are in fields 19–24
can_exit_opp: same for opponent
→ 2 values

// Rest corner status (field 12/13)
own_corner_taken: 1.0 if field 12 has ≥2 own checkers
opp_corner_taken: 1.0 if field 13 has ≥2 opponent checkers
→ 2 values

// Jan de 3 coups counter (normalized)
dice_roll_count_own: dice_roll_count / 3.0 (clamped to 1.0)
→ 1 value

Size: 204 + 8 + 2 + 2 + 1 = 217
Training benefit: Quarter fill status is what an expert player reads at a glance. Providing it explicitly can halve
the number of self-play games needed to learn the basic strategic structure. The corner status similarly removes
expensive inference from the network.

### Option C — Option B + richer positional features (flat 1D)

More complete, higher sample efficiency, minor extra cost.

Add on top of Option B:

// Per-quarter fill fraction — how close to filling each quarter
own_quarter_fill_fraction[q] (q=0..3): (count of fields with ≥2 own checkers in quarter q) / 6.0
opp_quarter_fill_fraction[q] (q=0..3): same for opponent
→ 8 values

// Blot counts — number of own/opponent single-checker fields globally
// (tells the network at a glance how much battage risk/opportunity exists)
own_blot_count: (number of own fields with exactly 1 checker) / 15.0
opp_blot_count: same for opponent
→ 2 values

// Bredouille would-double multiplier (already present, but explicitly scaled)
// No change needed, already binary

Size: 217 + 8 + 2 = 227
Tradeoff: The fill fractions are partially redundant with the TD-Gammon per-field counts, but they save the network
from summing across a quarter. The redundancy is not harmful (it gives explicit shortcuts).

### Option D — 2D spatial tensor {K, 24}

For CNN-based networks. Best eventual architecture but requires changing the training setup.

Shape {14, 24} — 14 feature channels over 24 field positions:

Channel 0: own_count_1 (blot)
Channel 1: own_count_2
Channel 2: own_count_3
Channel 3: own_count_overflow (float)
Channel 4: opp_count_1
Channel 5: opp_count_2
Channel 6: opp_count_3
Channel 7: opp_count_overflow
Channel 8: own_corner_mask (1.0 at field 12)
Channel 9: opp_corner_mask (1.0 at field 13)
Channel 10: final_quarter_mask (1.0 at fields 19–24)
Channel 11: quarter_filled_own (constant 1.0 across the 6 fields of any filled own quarter)
Channel 12: quarter_filled_opp (same for opponent)
Channel 13: dice_reach (1.0 at fields reachable this turn by own checkers)

Global scalars (dice, scores, bredouille, etc.) embedded as extra all-constant channels, e.g. one channel with uniform
value dice1/6.0 across all 24 positions, another for dice2/6.0, etc. Alternatively pack them into a leading "global"
row by returning shape {K, 25} with position 0 holding global features.

Size: 14 × 24 + few global channels ≈ 336–384
C++ change needed: ObservationTensorShape() → {14, 24} (or {kNumChannels, 24}), kStateEncodingSize updated
accordingly.
Training setup change needed: The AlphaZero config must specify a ResNet/ConvNet rather than an MLP. OpenSpiel's
alpha_zero.cc uses CreateTorchResnet() which already handles 2D input when the tensor shape has 3 dimensions ({C, H,
W}). Shape {14, 24} would be treated as 2D with a 1D spatial dimension.
Benefit: A convolutional network with kernel size 6 (= quarter width) would naturally learn quarter patterns. Kernel
size 2–3 captures adjacent-field "tout d'une" interactions.

### On 3D tensors

Shape {K, 4, 6} — K features × 4 quarters × 6 fields — is the most semantically natural for Trictrac. The quarter is
the fundamental tactical unit. A 2D conv over this shape (quarters × fields) would learn quarter-level patterns and
field-within-quarter patterns jointly.

However, 3D tensors require a 3D convolutional network, which OpenSpiel's AlphaZero doesn't use out of the box. The
extra architecture work makes this premature unless you're already building a custom network. The information content
is the same as Option D.

### Recommendation

Start with Option B (217 values, flat 1D, kStateEncodingSize = 217). It requires only changes to to_vec() in Rust and
the one constant in the C++ header — no architecture changes, no training pipeline changes. The three additions
(quarter fill status, exit readiness, corner status) are the features a human expert reads before deciding their move.

Plan Option D as a follow-up once you have a baseline trained on Option B. The 2D spatial CNN becomes worthwhile when
the MCTS games-per-second is high enough that the limit shifts from sample efficiency to wall-clock training time.

Costs summary:

| Option  | Size | Rust change      | C++ change              | Architecture change | Expected sample-efficiency gain |
| ------- | ---- | ---------------- | ----------------------- | ------------------- | ------------------------------- |
| Current | 36   | —                | —                       | —                   | baseline                        |
| A       | 204  | to_vec() rewrite | constant update         | none                | moderate (color separation)     |
| B       | 217  | to_vec() rewrite | constant update         | none                | large (quarter fill explicit)   |
| C       | 227  | to_vec() rewrite | constant update         | none                | large + moderate                |
| D       | ~360 | to_vec() rewrite | constant + shape update | CNN required        | large + spatial                 |

One concrete implementation note: since get_tensor() in cxxengine.rs calls game_state.mirror().to_vec() for player 2,
the new to_vec() must express everything from the active player's perspective (which the mirror already handles for
the board). The quarter fill status and corner status should therefore be computed on the already-mirrored state,
which they will be if computed inside to_vec().

## Other algorithms

The recommended features (Option B) are the same or more important for DQN/PPO. But two things do shift meaningfully.

### 1. Without MCTS, feature quality matters more

AlphaZero has a safety net: even a weak policy network produces decent play once MCTS has run a few hundred
simulations, because the tree search compensates for imprecise network estimates. DQN and PPO have no such backup —
the network must learn the full strategic structure directly from gradient updates.

This means the quarter-fill status, exit readiness, and corner features from Option B are more important for DQN/PPO,
not less. With AlphaZero you can get away with a mediocre tensor for longer. With PPO in particular, which is less
sample-efficient than MCTS-based methods, a poorly represented state can make the game nearly unlearnable from
scratch.

### 2. Normalization becomes mandatory, not optional

AlphaZero's value target is bounded (by MaxUtility) and MCTS normalizes visit counts into a policy. DQN bootstraps
Q-values via TD updates, and PPO has gradient clipping but is still sensitive to input scale. With heterogeneous raw
values (dice 1–6, counts 0–15, booleans 0/1, points 0–12) in the same vector, gradient flow is uneven and training can
be unstable.

For DQN/PPO, every feature in the tensor should be in [0, 1]:

dice values: / 6.0
checker counts: overflow channel / 12.0
points: / 12.0
holes: / 12.0
dice_roll_count: / 3.0 (clamped)

Booleans and the TD-Gammon binary indicators are already in [0, 1].

### 3. The shape question depends on architecture, not algorithm

| Architecture                         | Shape                        | When to use                                                         |
| ------------------------------------ | ---------------------------- | ------------------------------------------------------------------- |
| MLP                                  | {217} flat                   | Any algorithm, simplest baseline                                    |
| 1D CNN (conv over 24 fields)         | {K, 24}                      | When you want spatial locality (adjacent fields, quarter patterns)  |
| 2D CNN (conv over quarters × fields) | {K, 4, 6}                    | Most semantically natural for Trictrac, but requires custom network |
| Transformer                          | {24, K} (sequence of fields) | Attention over field positions; overkill for now                    |

The choice between these is independent of whether you use AlphaZero, DQN, or PPO. It depends on whether you want
convolutions, and DQN/PPO give you more architectural freedom than OpenSpiel's AlphaZero (which uses a fixed ResNet
template). With a custom DQN/PPO implementation you can use a 2D CNN immediately without touching the C++ side at all
— you just reshape the flat tensor in Python before passing it to the network.

### One thing that genuinely changes: value function perspective

AlphaZero and ego-centric PPO always see the board from the active player's perspective (handled by mirror()). This
works well.

DQN in a two-player game sometimes uses a canonical absolute representation (always White's view, with an explicit
current-player indicator), because a single Q-network estimates action values for both players simultaneously. With
the current ego-centric mirroring, the same board position looks different depending on whose turn it is, and DQN must
learn both "sides" through the same weights — which it can do, but a canonical representation removes the ambiguity.
This is a minor point for a symmetric game like Trictrac, but worth keeping in mind.

Bottom line: Stick with Option B (217 values, normalized), flat 1D. If you later add a CNN, reshape in Python — there's no need to change the Rust/C++ tensor format. The features themselves are the same regardless of algorithm.
