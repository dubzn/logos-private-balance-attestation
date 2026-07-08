{
  description = "Private Balance Attestation Basecamp ui_qml module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/e9f00bd893984bc8ce46c895c3bf7cac95331127";
    logos-module-builder.url = "github:logos-co/logos-module-builder/b0e41abf3e14c0534b41933c5f8e3fc697319037";
    logos-module-builder.inputs.nixpkgs.follows = "nixpkgs";
    delivery_module.url = "github:logos-co/logos-delivery-module/v0.1.3";
  };

  outputs = inputs@{ logos-module-builder, ... }:
    logos-module-builder.lib.mkLogosQmlModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
    };
}
