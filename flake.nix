{
  description = "Time-Tracking Cube (TTC)";

  inputs = {
    # nixpkgs.url = "github:NixOS/nixpkgs/unstable";
    nixpkgs.url = "github:NixOS/nixpkgs/23.11";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, ... }@inputs: inputs.utils.lib.eachSystem [
    # Add the system/architecture you would like to support here. Note that not
    # all packages in the official nixpkgs support all platforms.
    "x86_64-linux"
  ]
    (system:
      let
        pkgs = import nixpkgs {
          inherit system;

          # Add overlays here if you need to override the nixpkgs
          # official packages.
          overlays = [ ];

          # Uncomment this if you need unfree software (e.g. cuda) for
          # your project.
          #
          config.allowUnfree = true;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # Development Tools
            cppcheck
            gcc-arm-embedded
            cmake
            gnumake
            zsh
          ];
        };
      });
}
