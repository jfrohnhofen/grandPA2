{
  description = "grandPA2 - STM32F4 Firmware & Windows Host Screen Scraper";

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

        # Shared base toolchain components
        baseRust = [
          fenix.packages.${system}.stable.cargo
          fenix.packages.${system}.stable.rustc
          fenix.packages.${system}.stable.rust-src
        ];

        # Toolchain for STM32F4 Firmware (thumbv7em-none-eabihf)
        firmwareToolchain = fenix.packages.${system}.combine (baseRust ++ [
          fenix.packages.${system}.targets.thumbv7em-none-eabihf.stable.rust-std
        ]);

        # Toolchain for Windows Host App (x86_64-pc-windows-gnu)
        hostToolchain = fenix.packages.${system}.combine (baseRust ++ [
          fenix.packages.${system}.targets.x86_64-pc-windows-gnu.stable.rust-std
        ]);
      in
      {
        devShells = {
          # Dev shell for firmware subproject
          firmware = pkgs.mkShell {
            buildInputs = [
              firmwareToolchain
              pkgs.rust-analyzer
              pkgs.cargo-binutils
              pkgs.probe-rs-tools
              pkgs.dfu-util
              pkgs.alsa-utils
              pkgs.python3
              pkgs.python3Packages.pillow
            ];

            shellHook = ''
              echo "STM32F4 Firmware Dev Shell"
              echo "Target: thumbv7em-none-eabihf"
            '';
          };

          # Dev shell for host subproject (Windows app cross-compilation / native build)
          host = pkgs.mkShell {
            buildInputs = [
              hostToolchain
              pkgs.rust-analyzer
              pkgs.pkg-config
              pkgs.alsa-lib
              pkgs.pkgsCross.mingwW64.stdenv.cc
              pkgs.pkgsCross.mingwW64.windows.pthreads
              pkgs.cargo-xwin
            ];

            CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "${pkgs.pkgsCross.mingwW64.stdenv.cc}/bin/x86_64-w64-mingw32-gcc";
            LD_LIBRARY_PATH = "${pkgs.alsa-lib}/lib";

            shellHook = ''
              echo "Windows Host App Dev Shell"
              echo "Target: x86_64-pc-windows-gnu / x86_64-pc-windows-msvc"
            '';
          };
        
          keycaps = pkgs.mkShell {
            # Packages available in the environment
            packages = with pkgs; [
              python3
              inkscape
            ];

            # Optional: commands to run when entering the shell
            shellHook = ''
              echo "🎨 SVG Generation Environment Loaded"
              echo "Python: $(python3 --version)"
              echo "Inkscape: $(inkscape --version | cut -d' ' -f2)"
            '';
          };
        };
      }
    );
}
