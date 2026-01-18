{ pkgs, ... }:

{

  packages = [

    # pour burn-rs
    pkgs.SDL2_gfx
    #  (compilation sdl2-sys)
    pkgs.cmake
    pkgs.libffi
    pkgs.wayland-scanner

    # dev tools
    pkgs.samply # code profiler
    pkgs.feedgnuplot # to visualize bots training results

    # --- AI training with python ---
    # generate python classes from rust code
    pkgs.maturin
    # required by python numpy
    pkgs.libz

    # for bevy
    pkgs.alsa-lib
    pkgs.udev

    # bevy fast compile
    pkgs.clang
    pkgs.lld

    # copié de https://github.com/mmai/Hyperspeedcube/blob/develop/devenv.nix
    # TODO : retirer ce qui est inutile
    # pour erreur à l'exécution, selon https://github.com/emilk/egui/discussions/1587
    pkgs.libxkbcommon
    pkgs.libGL

    # WINIT_UNIX_BACKEND=wayland
    pkgs.wayland

    # WINIT_UNIX_BACKEND=x11
    pkgs.xorg.libXcursor
    pkgs.xorg.libXrandr
    pkgs.xorg.libXi
    pkgs.xorg.libX11

    pkgs.vulkan-headers
    pkgs.vulkan-loader
    # ------------ fin copie

  ];

  # https://devenv.sh/languages/
  languages.rust.enable = true;


  # AI training with python
  enterShell = ''
    PYTHONPATH=$PYTHONPATH:$PWD/.devenv/state/venv/lib/python3/site-packages
  '';

  languages.python = {
    enable = true;
    uv.enable = true;
    venv.enable = true;
    venv.requirements = "
      pip
      gymnasium
      numpy
      stable-baselines3
      shimmy
    ";
  };

  # https://devenv.sh/scripts/
  # scripts.hello.exec = "echo hello from $GREET";

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";
}
