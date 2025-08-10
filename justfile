#!/usr/bin/env -S just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

doc:
  cargo doc --no-deps
shell:
	devenv shell
runcli:
	RUST_LOG=info cargo run --bin=client_cli
runclibots:
	cargo run --bin=client_cli -- --bot random,dqnburn:./models/burn_dqn_model.mpk
	#cargo run --bin=client_cli -- --bot dqn:./models/dqn_model_final.json,dummy
	# RUST_LOG=info cargo run --bin=client_cli -- --bot dummy,dqn
match:
  cargo build --release --bin=client_cli
  LD_LIBRARY_PATH=./target/release  ./target/release/client_cli -- --bot dummy,dqn
profile:
  echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
  cargo build --profile profiling
  samply record ./target/profiling/client_cli --bot dummy,dummy
pythonlib:
  maturin build -m store/Cargo.toml --release
  pip install --no-deps --force-reinstall --prefix .devenv/state/venv target/wheels/*.whl
trainsimple:
  cargo build --release --bin=train_dqn_simple
  LD_LIBRARY_PATH=./target/release ./target/release/train_dqn_simple | tee /tmp/train.out
trainbot:
  #python ./store/python/trainModel.py
  # cargo run --bin=train_dqn # ok
  cargo build --release --bin=train_dqn_burn
  LD_LIBRARY_PATH=./target/release  ./target/release/train_dqn_burn | tee /tmp/train.out
plottrainbot:
  cat /tmp/train.out | grep -v rolled | grep -v correct | awk -F '[ ,]' '{print $5}' | feedgnuplot --lines --points --unset grid --title 'adv = random ; density = 256 ; err_reward = -1 ; reward_ratio = 1 ; decay = 1500 ; max steps = 600' --terminal png > doc/trainbots_stats/train_random_256_1_1_1500_600.png
debugtrainbot:
  cargo build --bin=train_dqn_burn
  RUST_BACKTRACE=1 LD_LIBRARY_PATH=./target/debug  ./target/debug/train_dqn_burn
profiletrainbot:
  echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
  cargo build --profile profiling --bin=train_dqn_burn
  LD_LIBRARY_PATH=./target/profiling  samply record ./target/profiling/train_dqn_burn
