shell:
	devenv shell
	# nix develop
startserver:
	cargo run --bin=trictrac-server
startclient1:
	cargo run --bin=trictrac-client Titi
startclient2:
	cargo run --bin=trictrac-client Tutu
