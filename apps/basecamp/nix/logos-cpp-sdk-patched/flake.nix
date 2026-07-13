{
  description = "Temporary Logos C++ SDK bstr event payload fix";

  inputs = {
    upstream.url = "github:logos-co/logos-cpp-sdk/d12a7bbb45d7d05f003b5d746a6c4dbc9df28315";
    nixpkgs.follows = "upstream/nixpkgs";
  };

  outputs = { nixpkgs, upstream, ... }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
    in
    {
      packages = nixpkgs.lib.genAttrs systems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          original = upstream.packages.${system};
          patchedBin = original.logos-cpp-bin.overrideAttrs (old: {
            patches = (old.patches or [ ]) ++ [ ./fix-bstr-event-payload.patch ];
          });
          patchedSdk = pkgs.symlinkJoin {
            name = "logos-cpp-sdk-patched";
            paths = [
              patchedBin
              original.logos-cpp-lib
              original.logos-cpp-include
            ];
            propagatedBuildInputs = original.default.propagatedBuildInputs or [ ];
          };
        in
        original // {
          logos-cpp-bin = patchedBin;
          cpp-generator = patchedBin;
          logos-cpp-sdk = patchedSdk;
          default = patchedSdk;
        });
    };
}
