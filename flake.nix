{
  description = "Trictrac";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "i686-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
      nixpkgsFor = forAllSystems (system:
        import nixpkgs {
          inherit system;
          overlays = [ self.overlay ];
        }
      );
    in
    {
      overlay = final: prev:
        let
          # Extend final privately with rust-overlay to get rust-bin for the WASM
          # toolchain without exposing rust-overlay attributes to consumers.
          rustPkgs = final.extend rust-overlay.overlays.default;
        in
        {

          trictrac-front =
            let
              # WASM build needs wasm32-unknown-unknown target in the Rust toolchain
              rustToolchain = rustPkgs.rust-bin.stable.latest.default.override {
                targets = [ "wasm32-unknown-unknown" ];
              };
              rustPlatform = final.makeRustPlatform {
                cargo = rustToolchain;
                rustc = rustToolchain;
              };
              # Must match the wasm-bindgen version in Cargo.lock
              wasm-bindgen-version = "0.2.118";
              # wasm-bindgen-version = "0.2.121";
              # wasm-bindgen-cli = final.buildWasmBindgenCli rec {
              #   version = wasm-bindgen-version;
              #   src = final.fetchCrate {
              #     pname = "wasm-bindgen-cli";
              #     inherit version;
              #     hash = "sha256-ZOMgFNOcGkO66Jz/Z83eoIu+DIzo3Z/vq6Z5g6BDY/w=";
              #   };
              #   cargoDeps = rustPlatform.fetchCargoVendor {
              #     inherit src;
              #     name = "wasm-bindgen-cli-vendor";
              #     hash = "sha256-DPdCDPTAPBrbqLUqnCwQu1dePs9lGg85JCJOCIr9qjU=";
              #   };
              # };

              frontendCargoDeps = rustPlatform.fetchCargoVendor {
                src = ./.;
                name = "trictrac-frontend-vendor";
                hash = "sha256-neJh0ZQGa5LNY8vBu3kYkM+ARkXOW/EHx8sPBOsWsgE=";
              };
            in
            final.stdenv.mkDerivation {
              name = "trictrac-front";
              src = ./.;

              nativeBuildInputs = with final; [
                rustToolchain
                lld
                rustPlatform.cargoSetupHook
                wasm-bindgen-cli_0_2_118
                trunk
                binaryen
              ];

              cargoDeps = frontendCargoDeps;

              buildPhase = ''
                runHook preBuild
                export HOME=$TMPDIR

                # Pin tool versions so trunk finds them in PATH instead of downloading
                cat >> clients/web/Trunk.toml << 'EOF'

                [tools]
                wasm-bindgen = { version = "${wasm-bindgen-version}" }
                wasm-opt = { version = "version_124" }
                EOF

                pushd clients/web
                trunk build --release --offline
                popd

                runHook postBuild
              '';

              installPhase = ''
                runHook preInstall
                mkdir -p $out
                cp -R clients/web/dist/. $out/
                runHook postInstall
              '';
            };

          trictrac = with final; rustPlatform.buildRustPackage {
            pname = "trictrac";
            version = "0.2.12"; # trictrac-version
            src = ./.;

            nativeBuildInputs = [ pkg-config ];
            buildInputs = [ openssl ];

            # Build only the relay server; skip WASM/bot crates
            cargoBuildFlags = [ "-p" "relay-server" ];
            doCheck = false;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            postInstall = ''
              install -m 644 ${./server/relay-server/GameConfig.json} $out/GameConfig.json
            '';

            meta = with lib; {
              description = "A online game of trictrac";
              homepage = "https://github.com/mmai/trictrac";
              license = licenses.gpl3;
              platforms = platforms.unix;
            };
          };

          trictrac-docker = with final;
            let
              port = "8080";
              entrypoint = writeScript "entrypoint.sh" ''
                #!${runtimeShell}
                # Populate a writable working dir with static files + config
                mkdir -p /var/lib/trictrac
                for f in ${trictrac-front}/*; do
                  ln -sf "$f" "/var/lib/trictrac/$(basename "$f")"
                done
                cp -n ${trictrac}/GameConfig.json /var/lib/trictrac/ 2>/dev/null || true
                cd /var/lib/trictrac
                echo "Starting trictrac server on port ${port}"
                exec ${trictrac}/bin/relay-server
              '';
            in
            dockerTools.buildImage {
              name = "mmai/trictrac";
              tag = "latest";
              copyToRoot = buildEnv {
                name = "trictrac-env";
                paths = [ busybox ];
              };
              config = {
                Entrypoint = [ entrypoint ];
                ExposedPorts = {
                  "${port}/tcp" = { };
                };
              };
            };

        };

      packages = forAllSystems (system: {
        inherit (nixpkgsFor.${system}) trictrac trictrac-front trictrac-docker;
      });

      defaultPackage = forAllSystems (system: self.packages.${system}.trictrac);

      # trictrac service module
      nixosModule = import ./module.nix;

    };
}
