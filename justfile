#!/usr/bin/env -S just --justfile
# ^ A shebang isn't required, but allows a justfile to be executed
#   like a script, with `./justfile test`, for example.

doc:
  cargo doc --no-deps
shell:
	devenv shell
startserver:
	RUST_LOG=trictrac_server cargo run --bin trictrac-server
startclient1:
	RUST_LOG=trictrac_client cargo run --bin=trictrac-client Titi
startclient2:
	RUST_LOG=trictrac_client cargo run --bin=trictrac-client Titu
startclienttui:
	RUST_LOG=trictrac_client cargo run --bin=client_tui Tutu
