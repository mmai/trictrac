{ pkgs, ... }:

{
  # https://devenv.sh/basics/
  # env.GREET = "devenv";

  packages = [ 
    pkgs.alsaLib pkgs.udev # for bevy
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
