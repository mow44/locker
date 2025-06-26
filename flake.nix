{
  description = "locker";

  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    import-cargo.url = "github:edolstra/import-cargo";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      import-cargo,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        locker =
          let
            lastModifiedDate = self.lastModifiedDate or self.lastModified or "19700101";
            version = "${builtins.substring 0 8 lastModifiedDate}-${self.shortRev or "dirty"}";
          in
          {
            inShell ? false,
          }:
          pkgs.stdenv.mkDerivation rec {
            name = "locker-${version}";

            src = if inShell then null else ./.;

            buildInputs =
              with pkgs;
              [
                cargo
              ]
              ++ (
                if inShell then
                  [
                    lazygit
                  ]
                else
                  [
                    (import-cargo.builders.importCargo {
                      lockFile = ./Cargo.lock;
                      inherit pkgs;
                    }).cargoHome
                  ]
              );

            target = "--release";
            doCheck = true;

            buildPhase = "cargo build ${target} --frozen --offline";
            checkPhase = "cargo test ${target} --frozen --offline";
            installPhase = ''
              mkdir -p $out
              cargo install --frozen --offline --path . --root $out
              rm $out/.crates.toml
            '';
          };
      in
      {
        packages.default = locker { };
        devShells.default = locker { inShell = true; };
      }
    );
}
