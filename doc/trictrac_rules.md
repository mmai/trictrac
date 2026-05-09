# Trictrac Rules — Quick Reference

This document summarises the rules of grand trictrac based on the 2013 Malfilâtre edition ([full text](refs/laws_and_rules_of_trictrac.md)).

French terms follow the mapping in [vocabulary.md](refs/vocabulary.md).

---

## 1. Board and Starting Position

- 24 triangular fields (_flèches_ / _cases_), numbered 1–24 from each player's perspective.
- 4 quarters of 6 fields: **small jan** (1–6), **big jan** (7–12), **opponent's big jan** (13–18), **return jan** (19–24, exit zone).
- Field 12 (White) / 13 (Black) is the **rest corner** (_coin de repos_).
- Each player starts with all 15 checkers in a stack (_talon_) on field 1.
- Checkers always move in the same direction (White: 1→24; Black: mirror of that).

## 2. Dice and Movement

- Both dice are rolled together; both must be played if possible.
- If only one can be played and there is a choice, the higher number must be played.
- A single checker may play both dice successively — a **chained move** (_tout d'une_) — stopping on an intermediate resting field (_repos_) between the two dice.
- **Fields are single-color**: a checker may only land on an empty field or one already occupied by own checkers.
- Landing on a field with ≥ 1 opponent checker is **forbidden** (blocked field).
- An unplayed number is a **helpless man** (_jan-qui-ne-peut_): 2 points penalty per unplayed die, credited to the opponent.

## 3. The Rest Corner (Field 12 / 13)

- Must be entered **simultaneously** (_d'emblée_): exactly 2 checkers must enter together.
- Must be vacated simultaneously: exactly 2 checkers must leave together.
- Always holds ≥ 2 checkers while occupied; a single checker there is forbidden.
- Two ways to take the corner:
  - **By effect** (_par effet_): normal die values land exactly on it.
  - **By puissance** (_par puissance_): the opponent's corner is empty; the player could land exactly on the opponent's corner, but by privilege he takes their own instead (as if stepping back one field).
- If both by-effect and by-puissance are possible, by-effect takes priority.
- An empty corner may serve as a resting field during a chained move (not a landing).
- Placing checkers on the **opponent's** corner is always forbidden.

## 4. Scoring: Points and Holes

- Points are tracked with tokens (0–11); **12 points = 1 hole** (_trou_).
- A **hole won bredouille** (_bredouille_) counts as **2 holes**: the active player scored 12 consecutive points from zero without the opponent scoring anything in between. The second player to start marking takes a double token (the _pavillon_ / flag) and can also win bredouille.
- The ordinary game ends when one player reaches **12 holes**.

## 5. Scoring Events (Jans)

All point values: normal roll / double.

### 5a. Opening Jans (first rolls of a setting only)

| Jan                   | Condition                                                                                | Points                               |
| --------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------ |
| **Two tables jan**    | First 2 checkers deployed; roll covers both rest corners; opponent's corner is empty     | 4 / 6 (to player)                    |
| **Contre two tables** | Same, but opponent has already taken their corner                                        | 4 / 6 (to opponent, as false hit)    |
| **Mezeas jan**        | Corner just taken (2 checkers); next roll shows one or two aces; opponent's corner empty | 4 per ace / 6 for double (to player) |
| **Contre mezeas**     | Same, but opponent's corner is occupied                                                  | 4 / 6 (to opponent)                  |
| **Six tables jan**    | After 2 rolls a checker is on 4 of the first 6 fields; 3rd roll could complete all 6     | 4 (always; not possible on a double) |

### 5b. Jan Filling and Conserving

A jan is **full** (_plein_) when all 6 of its fields hold ≥ 2 own checkers.

- **Filling**: the last checker is brought in to complete the jan.
  - Up to 3 ways: each direct die value covering the last field, or the combined sum (chained move).
  - Each way: **4 / 6** points.
  - Doubles allow at most 2 ways.
  - "Filling in passing" (player must break the jan to play the other die) scores nothing.
- **Conserving**: both dice can be played without disturbing any of the 12 checkers of the full jan.
  - Worth **4 / 6** points (at most one way).
  - Conservation by helplessness (_par impuissance_): only die value 6 triggers this (smaller values can always be played within the jan by breaking it).
  - The full return jan may be conserved by exiting one or more checkers.
- After marking points for filling, the player **must** actually fill the jan with the appropriate checker(s) — failure is a false move and a school.

### 5c. Hitting (_Jan de Récompense_)

Hitting is **always fictitious**: a checker is "hit" when a die value could cover an opponent checker on a half-field (_demi-case_), but **no actual checker moves**. The opponent's checker stays.

**True hit**: a die (direct or combined sum) fictitiously covers the opponent checker.

- In the **small jan table** (fields 1–12): **4 / 6** points per way.
- In the **big jan table** (fields 13–24): **2 / 4** points per way.

Ways to hit:

- **1 way**: only one direct die, or only the combined sum, covers the checker.
- **2 ways**: both direct dice cover it, or one die + the combined sum.
- **3 ways**: both dice + the combined sum (requires a normal roll; doubles max at 2 ways).

**Combined-sum hit** requires a free **resting field** between the two dice stops: the field must be empty, own, or a single opponent checker (which is then also hit).

**False hit** (_à faux_): the combined sum could hit but no valid resting field exists (all intermediate options are full opponent fields). The opponent gains the points the player would have scored.

- True-hit points are always marked before false-hit points.
- A checker already hit truly cannot also be hit falsely in the same move.
- Multiple checkers may be hit simultaneously (some true, some false).

**Corner hit**: player holds their own corner; opponent's corner is empty; the dice could simultaneously take the opponent's corner. Worth **4 / 6** points. Never false.

### 5d. Exit

- When all 15 checkers are in the return jan (fields 19–24), the player may exit.
- The exit rail counts as one additional field value.
- **Exact exit**: die value brings the checker directly to the exit rail — allowed.
- **Overflow** (_nombre excédant_): die value would carry the farthest checker past the rail — must exit.
- **Failing number** (_nombre défaillant_): die cannot reach or overflow — must play within the jan.
- A player may choose not to use an exact exit value and play within the jan instead — but overflow must always exit.
- It is forbidden to deliberately play a die within the jan to force the second die to be played as an overflow (using a checker closer to the exit).
- When the last checker exits: **4 points** on a normal roll, **6 points** on a double.
- After exit: checkers reset; the player who exited keeps first-move privilege for the new setting.

## 6. Forbidden Jans

A player **may not** place a checker in the opponent's small jan or big jan as long as the opponent can still materially complete a full jan there (i.e., enough of their own checkers remain to fill it).

Exception: during a chained move, an empty field in the opponent's big jan (including their empty corner) may serve as a resting field to pass a checker into the return jan.

## 7. Sequence of Play

Each turn follows this order:

1. Mark opponent's helpless man points or contre-jans from the **previous** move.
2. Mark opponent's schools; rectify false moves if any.
3. **Roll dice.** Opponent may mark schools for steps 1–2.
4. Mark own points: opening jans, hits, fills, conserves, exit.
5. Decide to **stay** (_tenir_) or **leave** (_s'en aller_) if a hole was won on own roll.
6. If exiting: reset checkers, keep token positions, roll again.
7. Play both dice.

Points and holes must always be marked **before** touching checkers for the next move.

## 8. Staying or Leaving

After winning one or more holes on **own dice roll**, the player chooses:

- **Stay** (_tenir_): mark holes, reset opponent token to zero, mark remainder points, continue.
- **Leave** (_s'en aller_): announce it; opponent agrees or raises a fault. All checkers and tokens reset to zero; only holes remain. Player who left has first-move privilege in the new setting. Remainder points are forfeited; opponent scores nothing for that move.

If the winning points come from the **opponent's** roll (helpless man, schools), the player **must** stay — leaving is not an option.

---

## 9. Schools (Marking Penalties) — _Not Yet Implemented_

Schools are penalties for marking errors. They are worth exactly the number of points over- or under-marked on the faulty move. They are marked last in the turn sequence.

Key rules:

- A school is committed once dice are rolled or a token has been advanced too far and released.
- The opponent is never obliged to mark a school — but if they do, it must be marked in full.
- **False school**: incorrectly claiming a school — itself becomes a school for the opponent.
- **School escalation** (_augmentation d'école_): dispute over a school that escalates back and forth.
- No "school of school" exists (marking a school is never itself penalised).
- No school of holes for marking a bredouille hole as simple; a school of points applies for forgetting holes due to earned points.

---

## 10. The Scored Game (_Partie à Écrire_) — _Not Yet Implemented_

The scored game is played for an agreed number of **rounds** (_marqués_) and supports 2, 3, or 4 players (the 3/4-player format is called _chouette_).

### Goal of a Round

- A player must score at least **6 holes** and then **leave** to win the round.
- If both players are tied at ≥ 6 holes when one leaves, the round is **drawn** (_refait_) and replayed immediately.
- Winner of a round = player with the most holes after a leave.

### Bredouille in the Scored Game

- **Small bredouille** (_petite bredouille_): ≥ 6 consecutive holes → round counts **double**.
- **Big bredouille** (_grande bredouille_): ≥ 12 consecutive holes → round counts **quadruple**.
- The second player to score holes takes the flag (_pavillon_) at their peg. If the first player scores again, they take back the flag, cancelling both bredouilles.

### Payments

Each round is settled in tokens:

- Winner receives (winner's holes − loser's holes) tokens, plus **consolation** of 2 tokens.
- Small bredouille: each winner hole worth 2 tokens; consolation = 4.
- Big bredouille: each winner hole worth 4 tokens; consolation = 8.
- Loser holes always deducted at 1 token each.
- In 3-player games, the non-playing player also receives consolation from the loser.
- Replays double the consolation price each time.

A **queue** accumulates tokens from each defeat and is paid at game end to the player with the most tokens.

**Bets** (_paris_): rounds played beyond each player's average (the _contingent_). The first double-bet is the **postillon** (28 tokens, including 20 from the queue); each subsequent bet costs 8 tokens.

### Multi-Player Rotation (3 or 4 Players)

- 3 players: after each round, the winner is replaced by the third player; first-move privilege stays with the player who remained.
- 4 players: two teams of two; each player plays two rounds in a row then gives way to their partner.
- Non-active players may advise (opponents in 3-player, teammates in 4-player) but may not touch game components.

---

## 11. Implementation Status Summary

| Feature                                         | Status                         |
| ----------------------------------------------- | ------------------------------ |
| Board state, movement, rest corner              | Implemented                    |
| Helpless man                                    | Implemented                    |
| True / false hits (small jan, big jan, corner)  | Implemented                    |
| Jan filling and conserving (small, big, return) | Implemented                    |
| Opening jans (two tables, mezeas, six tables)   | Implemented                    |
| Exit and exit points                            | Implemented                    |
| Bredouille (hole bredouille)                    | Implemented (`can_bredouille`) |
| Forbidden jans                                  | Implemented                    |
| Stay / leave (_s'en aller_)                     | Implemented (`Go` event)       |
| Big bredouille (`can_big_bredouille`)           | Field exists, not used         |
| Schools                                         | Not implemented                |
| Scored game / rounds                            | Not implemented                |
| Misery pile (_pile de misère_)                  | Not implemented                |
