{
  description = "Desktop Entry Editor - A Slint-based .desktop file editor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        # Common build inputs
        buildInputs = with pkgs; [
          wayland
          libxkbcommon
          libGL
          fontconfig
          freetype
          expat
          libx11
          libxcursor
          libxi
          libxrandr
          libxcb
          vulkan-loader
          vulkan-headers
          systemd # for libudev
        ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
          makeWrapper
        ];

        # Cargo.toml + lock file for dependency derivation
        src = craneLib.path ./.;

        # Build just the cargo dependencies (cached)
        cargoArtifacts = craneLib.buildDepsOnly ({
          inherit src buildInputs nativeBuildInputs;
          # Don't fail on warnings during dep build
          CARGO_BUILD_RUSTFLAGS = "";
        });

        # Build the actual application
        desktop-entry-editor = craneLib.buildPackage ({
          inherit cargoArtifacts src buildInputs nativeBuildInputs;
        });

        # Wrapped binary with proper LD_LIBRARY_PATH
        desktop-entry-editor-wrapped = pkgs.runCommandLocal "desktop-entry-editor" {
          nativeBuildInputs = [ pkgs.makeWrapper ];
        } ''
          mkdir -p $out/bin
          makeWrapper ${desktop-entry-editor}/bin/desktop-entry-editor $out/bin/desktop-entry-editor \
            --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath buildInputs}"
        '';
      in
      {
        checks = {
          inherit desktop-entry-editor;
        };

        packages = {
          default = desktop-entry-editor-wrapped;
          inherit desktop-entry-editor;
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          packages = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer
            cargo-watch
            cargo-edit
          ];

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };
      });
}
