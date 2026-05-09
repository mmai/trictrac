{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
  # inputs.trictrac.url = "github:mmai/trictrac";
  inputs.trictrac.url = "..";

  outputs = { self, nixpkgs, trictrac }:
    {
      nixosConfigurations = {

        container = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";

          modules = [
            trictrac.nixosModule
            ({ pkgs, ... }:
              let
                hostname = "trictrac";
              in
              {
                boot.isContainer = true;

                # Let 'nixos-version --json' know about the Git revision
                # of this flake.
                system.configurationRevision = nixpkgs.lib.mkIf (self ? rev) self.rev;
                system.stateVersion = "25.11";

                # Network configuration.
                networking.useDHCP = false;
                networking.firewall.allowedTCPPorts = [ 80 ];
                networking.hostName = hostname;

                # trictrac.overlay already includes rust-overlay
                nixpkgs.overlays = [ trictrac.overlay ];

                services.trictrac = {
                  enable = true;
                  protocol = "http";
                  hostname = hostname;
                };

                environment.systemPackages = with pkgs; [ neovim ];
              })
          ];
        };

      };
    };
}
