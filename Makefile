shell:
	devenv shell
	# nix develop
startserver:
	RUST_LOG=trictrac_server cargo run --bin trictrac-server
startclient1:
	RUST_LOG=trictrac_client cargo run --bin=trictrac-client Titi
startclient2:
	RUST_LOG=trictrac_client cargo run --bin=trictrac-client Tutu
