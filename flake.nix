{
  description = "Simple Porkbun dynamic DNS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:winterqt/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }: flake-utils.lib.eachDefaultSystem
    (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        pkg = pkgs.callPackage (import ./nix/package.nix) {
          inherit (pkgs.darwin.apple_sdk.frameworks) Security;
        };
      in

      {
        packages.pbd = pkg;
        defaultPackage = pkg;

        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo ] ++
            pkgs.lib.optionals pkgs.stdenv.isDarwin
              (with pkgs.darwin.apple_sdk.frameworks; [ Security pkgs.libiconv ]);
        };
      }
    ) // {
    nixosModules.pbd = import ./nix/module.nix;
  };
}
