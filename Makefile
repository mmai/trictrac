shell:
	# devenv shell
	nix develop
startserver:
	cargo run --bin=server
startclient1:
	cargo run --bin=client Titi
startclient2:
	cargo run --bin=client Tutu
