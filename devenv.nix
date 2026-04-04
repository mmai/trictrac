{ inputs, pkgs, ... }:

let
  pkgs-cmake3 = import inputs.nixpkgs-cmake3 { system = pkgs.stdenv.system; };
in
{
  packages = [
    # for Leptos
    pkgs.trunk
    # pkgs.wasm-bindgen-cli_0_2_114

    # pour burn-rs
    pkgs.SDL2_gfx
    #  (compilation sdl2-sys)
    pkgs-cmake3.cmake
    pkgs.libxcb
    pkgs.libffi
    pkgs.wayland-scanner

    # dev tools
    pkgs.samply # code profiler
    pkgs.feedgnuplot # to visualize bots training results

  ];

  # https://devenv.sh/languages/
  languages.rust.enable = true;

  # https://devenv.sh/scripts/
  # scripts.hello.exec = "echo hello from $GREET";

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";
}
