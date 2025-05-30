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
	RUST_LOG=info cargo run --bin=client_cli -- --bot dummy,ai
profile:
  echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
  cargo build --profile profiling
  samply record ./target/profiling/client_cli --bot dummy,dummy
pythonlib:
  maturin build -m store/Cargo.toml --release
  pip install --no-deps --force-reinstall --prefix .devenv/state/venv target/wheels/*.whl
trainbot:
  #python ./store/python/trainModel.py
  cargo run --bin=train_dqn
