with import <nixpkgs> {};

let
  jq = pkgs.jq;
  rdkafka = (import ./rdkafka.nix {
    inherit stdenv fetchFromGitHub zlib perl pkgconfig python;
  });
  kafka = pkgs.apacheKafka;
  zookeeper = pkgs.zookeeper;
  gdb = pkgs.gdb;
  java = pkgs.jre8;
  zlib = pkgs.zlib;
  cargo = pkgs.cargo;
in stdenv.mkDerivation {

    name = "kafka-jq";
    src = ./.;

    # dependencies
    buildInputs = [ cargo jq gcc rdkafka kafka zookeeper gdb java zlib ];

    # how to build
    buildPhase = ''
      make cool
    '';

    # installing after build
    installPhase = ''
      mkdir -p $out/bin
      cp cool $out/bin/cool
    '';
}
