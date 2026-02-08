import trictrac_store

game = trictrac_store.TricTrac()
print(game.current_player_idx())
print(game.get_legal_actions(game.current_player_idx()))
