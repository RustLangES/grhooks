{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (baseSystem:
      let
        cargoManifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        architectures = import ./architectures.nix;

        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          system = baseSystem;
          inherit overlays;
        };
        lib = pkgs.lib;

        uniqueBy = filter: list:
            let
              aux = seen: remaining:
                if remaining == [] then []
                else
                  let
                    item = builtins.head remaining;
                    rest = builtins.tail remaining;
                  in
                  if builtins.elem (filter item) seen then
                    aux seen rest
                  else
                    [item] ++ aux (seen ++ [(filter item)]) rest;
            in aux [] list;

        mkCrossPkgs = { arch, os }: let
          cross = arch + "-" + os;
          crossSystem = lib.systems.elaborate cross;
        in import nixpkgs {
          inherit overlays;
          crossSystem = if cross != "x86_64-linux" then crossSystem else null;
          localSystem = baseSystem;
        };

        mkDevShell = { arch, os, ... }: pkgs.mkShell {
          packages = with pkgs; [ rustc cargo ];
          shellHook = ''
            echo "DevShell for grhooks ${arch}-${os}"
          '';
        };

        mkPackage = { arch, os, target, ... }: let
          crossPkgs = mkCrossPkgs { inherit arch os; };
        in pkgs.rustPlatform.buildRustPackage {
          pname = "grhooks";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          CARGO_BUILD_TARGET = target;
          HOST_CC = lib.optionalString (os != "windows") "${pkgs.stdenv.cc.nativePrefix}cc";
          TARGET_CC = lib.optionalString (os != "windows") "${crossPkgs.stdenv.cc.targetPrefix}cc";

          buildInputs = with pkgs; [ ]
            ++ lib.optionals (os == "linux") [ stdenv.cc  glibc ]
            ++ lib.optionals (os == "macos") [ clang darwin.apple_sdk.frameworks.CoreFoundation ];
        };

        containerPkg = variant: let
          pkg = mkPackage variant;
          dockerPlatform =
            if variant.arch == "x86_64" then "amd64"
            else if variant.arch == "aarch64" then "arm64"
            else if variant.arch == "armv7l" then "arm"
            else if variant.arch == "armv6l" then "arm"
            else if variant.arch == "i686" then "386"
            else throw "Unsupported arch: ${variant.arch}";
        in pkgs.dockerTools.buildLayeredImage rec {
          name = cargoManifest.package.name;
          tag = cargoManifest.package.version;
          created = "now";
          architecture = dockerPlatform;

          contents = [ pkg ];
          config.Cmd = ["/bin/${name}"];
        };

        generatedMatrixJson = builtins.toJSON (lib.flatten (map ({ arch, os, formats, ... }:
          map (format: {
            arch = arch;
            os = os;
            format = format;
            package = "${os}-${arch}";
            bundler = if format == "zip" then ".#toZip"
              else if format == "deb" then "github:NixOS/bundlers#toDEB"
              else if format == "rpm" then "github:NixOS/bundlers#toRPM"
              else ".#toTarball";
          }) formats
        ) architectures));

        archList = uniqueBy (i: i.arch) architectures;
        generatedArchJson = builtins.toJSON archList;
      in
      {
        devShells = lib.listToAttrs (map ({ arch, os, target, ... }: {
          name = "${os}-${arch}";
          value = mkDevShell { inherit arch os target; };
        }) architectures) // {
          # Default Devshell
          default = mkDevShell {
            arch = "x86_64";
            os = "linux";
            target = "x86_64-unknown-linux-gnu";
          };
        };

        packages = (lib.listToAttrs (map ({ arch, os, target, ... }: {
          name = "${os}-${arch}";
          value = mkPackage { inherit arch os target; };
        }) architectures)) // (pkgs.lib.listToAttrs (map ({arch, ...} @ args: {
          name = "image-${arch}";
          value = containerPkg args;
        }) archList));

        apps = {
          help = {
            type = "app";
            program = toString (pkgs.writeScript "help" ''
              #!/bin/sh
              echo ""
              echo "Welcome to Grhooks!"
              echo ""
              echo -e "\033[0;33mAvailable architectures:\033[0m"
              ${lib.concatMapStringsSep "\n" (arch: ''echo "  - ${arch}"'') (lib.lists.unique (map ({ arch, ... }: arch) architectures))}
              echo ""
              echo -e "\033[0;35mAvailable OS:\033[0m"
              ${lib.concatMapStringsSep "\n" (os: ''echo "  - ${os}"'') (lib.lists.unique (map ({ os, ... }: os) architectures))}
              echo ""
              echo -e "\033[0;32mTo build docker image, use:\033[0m"
              echo "  nix build .#image-<arch>"
              echo -e "\033[0;32mTo build a specific variant, use:\033[0m"
              echo "  nix build .#<os>-<arch>"
              echo ""
              echo -e "\033[0;32mExample:\033[0m"
              echo "  nix build .#linux-x86_64"
            '');
          };

          matrix = {
            type = "app";
            program = toString (pkgs.writeScript "generate-matrix" ''
              #!/bin/sh
              echo '${generatedMatrixJson}'
            '');
          };

          archs = {
            type = "app";
            program = toString (pkgs.writeScript "generate-arch-list" ''
              #!/bin/sh
              echo '${generatedArchJson}'
            '');
          };
        };

        bundlers = let
          compress = ext: name: drv: pkgs.stdenv.mkDerivation {
            name = "${name}-${drv.name}";
            buildInputs = with pkgs; [ouch];
            unpackPhase = "true";
            buildPhase = ''
              export HOME=$PWD
              mkdir -p ./nix/store/
              mkdir -p ./bin
              for item in "$(cat ${pkgs.referencesByPopularity drv})"
              do
                cp -r $item ./nix/store/
              done

              cp -r ${drv}/bin/* ./bin/

              chmod -R a+rwx ./nix
              chmod -R a+rwx ./bin
              ouch c nix bin ${drv.name}.${ext}
            '';

            installPhase = ''
              mkdir -p $out
              cp -r *.${ext} $out
            '';
          };
        in {
          toTarball = compress "tar.xz" "tar-simple";
          toZip = compress "zip" "zip-simple";
        };
      });
}
