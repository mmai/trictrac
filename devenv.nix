{ pkgs, ... }:

{
  # https://devenv.sh/basics/
  # env.GREET = "devenv";

  packages = [ 
    # for bevy
    pkgs.alsaLib
    pkgs.udev

    # bevy fast compile
    pkgs.clang pkgs.lld

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

    pkgs.vulkan-headers pkgs.vulkan-loader
    # ------------ fin copie

  ];

  # enterShell = ''
  #   hello
  #   git --version
  # '';

  # https://devenv.sh/languages/
  languages.rust.enable = true;

  # https://devenv.sh/scripts/
  # scripts.hello.exec = "echo hello from $GREET";

  # https://devenv.sh/pre-commit-hooks/
  pre-commit.hooks.shellcheck.enable = true;

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";
}
