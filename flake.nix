{
  description = "Development shell for the comtains Rust byte-set project";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
        inherit (pkgs) lib;

        rustPackages = with pkgs; [
          rustc
          cargo
          rustfmt
          clippy
        ];

        baseInputs = with pkgs; [
          pkg-config
          openssl
          zlib
          cmake
          python3
        ];

        valgrindPackage =
          if (system == "aarch64-darwin") then null else pkgs.valgrind;
      in {
        devShells.default = pkgs.mkShell {
          buildInputs =
            rustPackages
            ++ baseInputs
            ++ lib.optional (valgrindPackage != null) valgrindPackage;

          shellHook = ''
            export CARGO_HOME="''${CARGO_HOME:-$PWD/.cargo}"
            export PATH="$CARGO_HOME/bin:$PATH"

            if [ "${system}" = "aarch64-darwin" ]; then
              echo "⚠️  Valgrind/Callgrind is unavailable on Apple Silicon macOS."
              echo "   The iai-callgrind benchmarks will compile but cannot be executed."
            else
              if ! command -v iai-callgrind-runner >/dev/null 2>&1; then
                echo "Installing iai-callgrind-runner (once per shell) ..."
                cargo install --locked --version 0.15.2 iai-callgrind-runner
              fi
              export IAI_CALLGRIND_RUNNER="$(command -v iai-callgrind-runner)"
              echo "IAI_CALLGRIND_RUNNER=$IAI_CALLGRIND_RUNNER"
            fi
          '';
        };
      });
}
