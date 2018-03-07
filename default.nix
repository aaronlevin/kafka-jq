# with import <nixpkgs> {};
let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  jq = nixpkgs.jq;
  rdkafka = (import ./rdkafka.nix {
    stdenv = nixpkgs.stdenv;
    fetchFromGitHub = nixpkgs.fetchFromGitHub;
    zlib = nixpkgs.zlib;
    perl = nixpkgs.perl;
    pkgconfig = nixpkgs.pkgconfig;
    python = nixpkgs.python;
  });
  gcc = nixpkgs.gcc;
  kafka = nixpkgs.apacheKafka;
  zookeeper = nixpkgs.zookeeper;
  gdb = nixpkgs.gdb;
  java = nixpkgs.jre8;
  zlib = nixpkgs.zlib;
  rust = nixpkgs.latest.rustChannels.nightly.rust;
in nixpkgs.stdenv.mkDerivation {

    name = "kafka-jq";
    src = ./.;

    # dependencies
    buildInputs = [ rust jq gcc rdkafka kafka zookeeper gdb java zlib ];

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
