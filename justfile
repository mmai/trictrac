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
	RUST_LOG=info cargo run --bin=client_cli -- --bot dqn,dummy
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
trainbot:
  #python ./store/python/trainModel.py
  # cargo run --bin=train_dqn # ok
  # cargo run --bin=train_dqn_burn # utilise debug (why ?)
  cargo build --release --bin=train_dqn_burn
  LD_LIBRARY_PATH=./target/release  ./target/release/train_dqn_burn | tee /tmp/train.out
plottrainbot:
  cat /tmp/train.out | awk -F '[ ,]' '{print $5}' | feedgnuplot --lines --points --unset grid
  #tail -f /tmp/train.out | awk -F '[ ,]' '{print $5}' | feedgnuplot --lines --points --unset grid
debugtrainbot:
  cargo build --bin=train_dqn_burn
  RUST_BACKTRACE=1 LD_LIBRARY_PATH=./target/debug  ./target/debug/train_dqn_burn
profiletrainbot:
  echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
  cargo build --profile profiling --bin=train_dqn_burn
  LD_LIBRARY_PATH=./target/profiling  samply record ./target/profiling/train_dqn_burn
