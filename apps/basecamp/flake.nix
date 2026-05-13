{
  description = "Private Balance Attestation Basecamp ui_qml module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system:
          f {
            pkgs = import nixpkgs { inherit system; };
          });
    in
    {
      packages = forAllSystems ({ pkgs }: {
        default = pkgs.stdenv.mkDerivation {
          pname = "balance_attestation";
          version = "0.1.0";
          src = ./.;

          nativeBuildInputs = [ pkgs.cmake pkgs.qt6.wrapQtAppsHook ];
          buildInputs = [ pkgs.qt6.qtbase pkgs.qt6.qtdeclarative pkgs.qt6.qtremoteobjects ];

          installPhase = ''
            mkdir -p $out/balance_attestation/src/qml
            cp $src/metadata.json $out/balance_attestation/
            cp $src/src/qml/BalanceAttestation.qml $out/balance_attestation/src/qml/
            if [ -f libbalance_attestation_plugin.dylib ]; then
              cp libbalance_attestation_plugin.dylib $out/balance_attestation/
            fi
            if [ -f libbalance_attestation_plugin.so ]; then
              cp libbalance_attestation_plugin.so $out/balance_attestation/
            fi
          '';
        };
      });
    };
}
