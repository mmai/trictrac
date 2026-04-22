#!/usr/bin/env -S just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

doc:
  cargo doc --no-deps
shell:
	devenv shell
runcli:
	RUST_LOG=info cargo run --bin=client_cli

[working-directory: 'clients/web-game']
dev-game:
  trunk serve

[working-directory: 'clients/web-game']
build-game:
  trunk build --release
  cp dist/index.html deploy/trictrac.html
  cp dist/*.wasm deploy/
  cp dist/*.js deploy/
  cp dist/*.css deploy/

[working-directory: 'deploy']
run-relay:
  ./relay-server

[working-directory: 'clients/web-user-portal']
dev-portal:
  trunk serve

[working-directory: 'clients/web-user-portal']
build-portal:
  trunk build --release
  cp dist/index.html ../../deploy/portal.html
  cp dist/*.wasm ../../deploy/
  cp dist/*.js ../../deploy/
  cp dist/*.css ../../deploy/

build-relay:
  CARGO_PROFILE_RELEASE_OPT_LEVEL=3 cargo build -p relay-server --release
  mkdir -p deploy
  cp target/release/relay-server deploy

runclibots:
	cargo run --bin=client_cli -- --bot random,dqnburn:./bot/models/burnrl_dqn_40.mpk
	#cargo run --bin=client_cli -- --bot dqn:./bot/models/dqn_model_final.json,dummy
	# RUST_LOG=info cargo run --bin=client_cli -- --bot dummy,dqn
match:
  cargo build --release --bin=client_cli
  LD_LIBRARY_PATH=./target/release  ./target/release/client_cli -- --bot dummy,dqn
profile:
  echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
  cargo build --profile profiling
  samply record ./target/profiling/client_cli --bot dummy,dummy
trainbot algo:
  # cargo run --bin=train_dqn # ok
  # ./bot/scripts/trainValid.sh
  ./bot/scripts/train.sh {{algo}}
plottrainbot algo:
  ./bot/scripts/train.sh plot {{algo}}
debugtrainbot:
  cargo build --bin=train_dqn_burn
  RUST_BACKTRACE=1 LD_LIBRARY_PATH=./target/debug  ./target/debug/train_dqn_burn
profiletrainbot:
  echo '1' | sudo tee /proc/sys/kernel/perf_event_paranoid
  cargo build --profile profiling --bin=train_dqn_burn
  LD_LIBRARY_PATH=./target/profiling  samply record ./target/profiling/train_dqn_burn

