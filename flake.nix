{
  description = "Rust firmware for STM32F4 (Black Pill - STM32F401 / STM32F411)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, fenix, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Toolchain with thumbv7em-none-eabihf target
        toolchain = fenix.packages.${system}.combine [
          fenix.packages.${system}.stable.cargo
          fenix.packages.${system}.stable.rustc
          fenix.packages.${system}.stable.rust-src
          fenix.packages.${system}.targets.thumbv7em-none-eabihf.stable.rust-std
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            toolchain
            pkgs.rust-analyzer
            pkgs.cargo-binutils
            pkgs.probe-rs-tools
            pkgs.dfu-util
            pkgs.alsa-utils
            pkgs.python3
            pkgs.python3Packages.pillow
          ];

          shellHook = ''
            echo "STM32F4 Rust Development Environment"
            echo "Target: thumbv7em-none-eabihf"
          '';
        };
      }
    );
}
