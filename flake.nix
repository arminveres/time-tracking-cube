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
            zsh
            # Development Tools
            cppcheck
            gcc-arm-embedded-13
            cmake
            gnumake
            picotool
            minicom
            mold
            clang-tools
          ];
          shellHook = ''
            export AEL_TOOLCHAIN_PATH="${pkgs.gcc-arm-embedded-13}/bin"
            export PICO_SDK_PATH="$(pwd)/firmware/libs/pico-sdk"
          '';
        };
      });
}
