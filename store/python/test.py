import trictrac

game = trictrac.TricTrac()
print(game.get_state())  # "Initial state"

moves = game.get_available_moves()
print(moves)  # [(0, 5), (3, 8)]

game.play_move(0, 5)
