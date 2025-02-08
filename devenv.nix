{ pkgs, ... }:

{

  packages = [

    # dev tools
    pkgs.samply # code profiler

    # generate python classes  from rust code (for AI training)
    pkgs.maturin
    # required to manually install generated python module in local venv
    pkgs.python312Packages.pip

    # required by python numpy (for AI training)
    pkgs.libz

    # for bevy
    pkgs.alsaLib
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

  enterShell = ''
    PYTHONPATH=$PYTHONPATH:$PWD/.devenv/state/venv/lib/python3.12/site-packages
  '';

  # https://devenv.sh/languages/
  languages.rust.enable = true;


  # for AI training
  languages.python = {
    enable = true;
    uv.enable = true;
    venv.enable = true;
    venv.requirements = "
      gym
      numpy
      stable-baselines3
    ";
  };

  # https://devenv.sh/scripts/
  # scripts.hello.exec = "echo hello from $GREET";

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";
}
