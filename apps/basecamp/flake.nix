{
  description = "Private Balance Attestation Basecamp ui_qml module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/e9f00bd893984bc8ce46c895c3bf7cac95331127";
    logos-cpp-sdk-patched.url = "path:./nix/logos-cpp-sdk-patched";
    logos-module-builder.url = "github:logos-co/logos-module-builder/92ef691ea72844134f6c68fb447d37f855fc9690";
    logos-module-builder.inputs.nixpkgs.follows = "nixpkgs";
    logos-module-builder.inputs.logos-cpp-sdk.follows = "logos-cpp-sdk-patched";
    delivery_module.url = "github:logos-co/logos-delivery-module/c21ffb83b2b891843de9a940dd60e5e56c8803de";
    delivery_module.inputs.logos-module-builder.follows = "logos-module-builder";
  };

  outputs = inputs@{ delivery_module, logos-module-builder, ... }:
    let
      moduleOutputs = logos-module-builder.lib.mkLogosQmlModule {
        src = ./.;
        configFile = ./metadata.json;
        flakeInputs = inputs;
      };
    in
    moduleOutputs // {
      packages = builtins.mapAttrs
        (system: packages: packages // {
          delivery-install = delivery_module.packages.${system}.install;
        })
        moduleOutputs.packages;
    };
}
