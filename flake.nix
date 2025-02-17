{
  description = "Time-Tracking Cube (TTC)";

  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  # Add the system/architecture you would like to support here. Note that not
  # all packages in the official nixpkgs support all platforms.
  outputs = inputs@{ self, nixpkgs, ... }:
    inputs.utils.lib.eachSystem [ "x86_64-linux" "aarch64-darwin" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ ];
          config.allowUnfree = true;
        };
      in {
        devShells.default = pkgs.mkShellNoCC {
          packages = with pkgs; [
            gcc-arm-embedded
            minicom
            mold-wrapped
            probe-rs
            gdb
            pkg-config
            sccache
          ];
          shellHook = "";
        };
      });
}
