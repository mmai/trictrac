
{
  description = "Trictrac";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
  flake-utils.lib.eachDefaultSystem (system:
        # let pkgs = nixpkgs.legacyPackages.${system}; in
        let pkgs = import nixpkgs {
          inherit system;
          config = { allowUnfree = true; };
        }; in
        {
          # devShell = import ./shell.nix { inherit pkgs; };
          devShell = with pkgs; mkShell rec {

            nativeBuildInputs = [
              pkg-config
              llvmPackages.bintools # To use lld linker
            ];

            buildInputs = [
              cargo rustc rustfmt rustPackages.clippy # rust
              # pre-commit

              alsa-lib udev
              vulkan-loader # needed for GPU acceleration
              xlibsWrapper xorg.libXcursor xorg.libXrandr xorg.libXi # To use x11 feature
              # libxkbcommon wayland # To use wayland feature
            ];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;

            shellHook = ''
            export HOST=127.0.0.1
            export PORT=7000
            '';
          };
        }
        );
      }
