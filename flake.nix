{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
      in
      {
        defaultPackage = naersk-lib.buildPackage ./.;
        devShell = with pkgs; mkShell {
          nativeBuildInputs = [ rustPlatform.bindgenHook ];
          buildInputs = [ cargo rustc rustfmt pre-commit rustPackages.clippy soapysdr-with-plugins  pkg-config libclang alsa-lib  libbladeRF ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      });
}
