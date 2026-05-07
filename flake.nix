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
      # rust-overlay must be applied before self.overlay so that rust-bin is available
      nixpkgsFor = forAllSystems (system:
        import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default self.overlay ];
        }
      );
    in
    {
      overlay = final: prev: {

        trictrac-front =
          let
            # WASM build needs wasm32-unknown-unknown target in the Rust toolchain
            rustToolchain = final.rust-bin.stable.latest.default.override {
              targets = [ "wasm32-unknown-unknown" ];
            };
            rustPlatform = final.makeRustPlatform {
              cargo = rustToolchain;
              rustc = rustToolchain;
            };
            # Must match the wasm-bindgen version in Cargo.lock
            wasm-bindgen-version = "0.2.118";
            wasm-bindgen-cli = final.buildWasmBindgenCli rec {
              version = wasm-bindgen-version;
              src = final.fetchCrate {
                pname = "wasm-bindgen-cli";
                inherit version;
                hash = "sha256-ve783oYH0TGv8Z8lIPdGjItzeLDQLOT5uv/jbFOlZpI=";
              };
              cargoDeps = rustPlatform.fetchCargoVendor {
                inherit src;
                name = "wasm-bindgen-cli-vendor";
                hash = "sha256-EYDfuBlH3zmTxACBL+sjicRna84CvoesKSQVcYiG9P0=";
              };
            };

            frontendCargoDeps = rustPlatform.fetchCargoVendor {
              src = ./.;
              name = "trictrac-frontend-vendor";
              hash = "sha256-W2xlFgmA8biiIaE/EbC7ebHryo1lzrQYdrOCp5Xxjn8=";
            };
          in
          final.stdenv.mkDerivation {
            name = "trictrac-front";
            src = ./.;

            nativeBuildInputs = with final; [
              rustToolchain
              lld
              rustPlatform.cargoSetupHook
              wasm-bindgen-cli
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
          version = "0.2.0";
          src = ./.;

          nativeBuildInputs = [ pkg-config ];
          buildInputs = [ openssl ];

          # Build only the relay server; skip WASM/bot crates
          cargoBuildFlags = [ "-p" "relay-server" ];
          doCheck = false;

          cargoLock = {
            lockFile = ./Cargo.lock;
            # Run `nix build .#trictrac` with the fake hashes to get the correct ones
            outputHashes = {
              "burn-rl-0.1.0" = "sha256-XAdabwHaSqi7ldO0v8Tuj7h1EX5QBeDIUgmme2Rdzqo=";
              "gym-rs-0.3.1" = "sha256-TA7sK027dbpWcsMLt+c+ggIZb0ZZvTk/e5ihvUYxmK0=";
            };
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
