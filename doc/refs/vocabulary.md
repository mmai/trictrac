# Trictrac Vocabulary — French / English

This table maps the French game terminology to the English terms used in this codebase (primarily the `store` crate). Where a code identifier exists, it is shown in `monospace`.

| French                                 | English (code)       | Notes                                                                                                    |
| -------------------------------------- | -------------------- | -------------------------------------------------------------------------------------------------------- |
| tablier                                | board                | `Board`                                                                                                  |
| case / flèche                          | field                | `Field` (1–24, 0 = exit); "flèche" (arrow) and "case" both refer to a field/point                        |
| demi-case                              | half-field           | A field occupied by exactly one checker                                                                  |
| dame                                   | checker              | `Checker`; a playing piece                                                                               |
| talon                                  | stack                | The starting pile of 15 checkers before they are deployed                                                |
| coin de repos / coin                   | rest corner / corner | `corner`; field 12 (White) or 13 (Black)                                                                 |
| bande de départ                        | starting rail        | The side rail where stacks start; holds the pegs and flag                                                |
| bande de sortie                        | exit rail            | Same rail, used as an extra field value during exit                                                      |
| petit jan                              | small jan            | Fields 1–6; `is_field_in_small_jan`                                                                      |
| grand jan                              | big jan              | Fields 7–12 (White's side, opponent's near zone)                                                         |
| jan de retour                          | return jan           | Fields 19–24; same fields as opponent's small jan ; where checkers gather before exiting; `last quarter` |
| table des petits jans                  | small jan table      | The board half containing both players' small jans (fields 1–12)                                         |
| table des grands jans                  | big jan table        | The board half containing both players' big jans (fields 13–24)                                          |
| plein (d'un jan)                       | filled (jan)         | All 6 fields of a jan hold ≥ 2 checkers                                                                  |
| remplir                                | fill                 | Scoring event: completing the fill of a jan; `FilledQuarter`                                             |
| conserver                              | conserve             | Scoring event: maintaining a filled jan without breaking it; `FilledQuarter`                             |
| jan de récompense — battre à vrai      | true hit             | `TrueHitSmallJan`, `TrueHitBigJan`, `TrueHitOpponentCorner`                                              |
| jan de récompense — battre à faux      | false hit            | `FalseHitSmallJan`, `FalseHitBigJan`                                                                     |
| batterie du coin                       | corner hit           | `TrueHitOpponentCorner`; hitting the opponent's empty rest corner                                        |
| jan-qui-ne-peut / impuissance          | helpless man         | `HelplessMan`; a die value that cannot be played (penalty for opponent)                                  |
| jan de deux tables                     | two tables jan       | `TwoTables`                                                                                              |
| contre-jan de deux tables              | contre two tables    | `ContreTwoTables`                                                                                        |
| jan de mézéas                          | mezeas jan           | `Mezeas`                                                                                                 |
| contre-jan de mézéas                   | contre mezeas        | `ContreMezeas`                                                                                           |
| jan de six tables / jan de trois coups | six tables jan       | `SixTables`; also called "three-roll jan"                                                                |
| sortie (première)                      | first player to exit | `FirstPlayerToExit`                                                                                      |
| sortie (nombre sortant)                | exit (exact exit)    | Moving a checker off the board with an exact die value                                                   |
| nombre excédant                        | overflow number      | Die value exceeding the checker's distance to the exit rail                                              |
| nombre défaillant                      | failing number       | A die value that cannot be played within the jan                                                         |
| tout d'une                             | chained move         | `chained move`; one checker playing both dice successively                                               |
| repos (case de repos)                  | rest (resting field) | An intermediate field where a checker pauses in a chained move                                           |
| doublet                                | double               | `is_double`; both dice show the same value                                                               |
| dé / dés                               | die / dice           | `Dice`                                                                                                   |
| cornet                                 | dice cup             | —                                                                                                        |
| par puissance                          | by puissance         | `is_move_by_puissance`; taking own corner using opponent's empty corner as virtual step                  |
| par effet                              | by effect            | `can_take_corner_by_effect`; taking own corner by normal die values                                      |
| d'emblée                               | simultaneously       | Two checkers entering (or leaving) the corner at the same time                                           |
| dédoubler                              | unstack corner       | Using one of the two corner-holding checkers (forbidden for corner exits)                                |
| trou / jeu                             | hole                 | `holes`; 12 points = 1 hole; the primary scoring unit                                                    |
| fichet                                 | peg                  | Physical marker tracking holes won along the board edge                                                  |
| jeton                                  | token                | Physical marker tracking points within a game (0–12)                                                     |
| pavillon                               | flag                 | The bredouille marker taken by the second player to score                                                |
| bredouille                             | bredouille           | `can_bredouille`; winning a hole while opponent scored nothing                                           |
| petite bredouille                      | small bredouille     | Winning a round (marqué) with ≥ 6 consecutive holes                                                      |
| grande bredouille                      | big bredouille       | `can_big_bredouille`; winning a round with ≥ 12 consecutive holes                                        |
| relevé                                 | new setting          | Resetting checkers to their stacks after a hole or exit                                                  |
| primauté                               | first-move privilege | The right to roll first, held by the player who exited or left first                                     |
| s'en aller                             | leave / go           | `Go` event; choosing to start a new setting after winning a hole                                         |
| tenir                                  | stay / hold          | Choosing to continue after winning a hole instead of leaving                                             |
| marqué                                 | round                | A scoring round in the "partie à écrire"                                                                 |
| partie ordinaire                       | ordinary game        | First to 12 holes wins                                                                                   |
| partie à écrire                        | scored game          | Multi-round game played for tokens                                                                       |
| à la chouette                          | chouette             | Three- or four-player format                                                                             |
| refait                                 | replay               | A drawn round (equal holes) that must be replayed                                                        |
| consolation                            | consolation          | Bonus tokens paid to the winner and, in 3-player games, the non-playing player                           |
| postillon                              | postillon            | The first "double bet" in final payment settlement                                                       |
| école                                  | school               | `schools`; a penalty for a marking error; opponent scores the missed points                              |
| fausse case                            | false move           | Playing a checker to the wrong field                                                                     |
| fausse école                           | false school         | Incorrectly claiming or marking a school penalty                                                         |
| augmentation d'école                   | school escalation    | Back-and-forth dispute over a school penalty                                                             |
| pile de misère                         | misery pile          | A special scoring configuration (not yet implemented in the codebase)                                    |
