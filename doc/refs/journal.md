# Journal

```sh
devenv init
cargo init
cargo add pico-args
```

Organisation store / server / client selon <https://herluf-ba.github.io/making-a-turn-based-multiplayer-game-in-rust-01-whats-a-turn-based-game-anyway>

_store_ est la bibliothèque contenant le _reducer_ qui transforme l'état du jeu en fonction des évènements. Elle est utilisée par le _server_ et le _client_. Seuls les évènements sont transmis entre clients et serveur.

## Organisation du store

lib

- game::GameState
  - error
  - dice
  - board
    - user
  - user

## Algorithme de détermination des coups

- strategy::choose_move
  - GameRules.get_possible_moves_sequences(with_excedents: bool)
    - get_possible_moves_sequences_by_dices(dice_max, dice_min, with_excedents, false);
    - get_possible_moves_sequences_by_dices(dice_min, dice_max, with_excedents, true);
      - has_checkers_outside_last_quarter() ok
      - board.get_possible_moves ok
      - check_corner_rules(&(first_move, second_move)) ok

- handle_event
  - state.validate (ok)
    - rules.moves_follow_rules (ok)
      - moves_possible ok
      - moves_follows_dices ok
      - moves_allowed (ok)
        - check_corner_rules ok
        - can_take_corner_by_effect ok
        - get_possible_moves_sequences -> cf. l.15
        - check_exit_rules
          - get_possible_moves_sequences -> cf l.15
        - get_quarter_filling_moves_sequences
          - get_possible_moves_sequences -> cf l.15
  - state.consume (RollResult) (ok)
    - get_rollresult_jans -> points_rules.get_result_jans (ok)
      - get_jans (ok)
        - get_jans_by_ordered_dice (ok)
          - get_jans_by_ordered_dice ( dices.poped )
        - move_rules.get_scoring_quarter_filling_moves_sequences (ok)
          - get_quarter_filling_moves_sequences cf l.8 (ok)
          - board.get_quarter_filling_candidate -> is_quarter_fillable ok
        - move_rules.get_possible_moves_sequence -> cf l.15
      - get_jans_points -> jan.get_points ok
