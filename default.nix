# with import <nixpkgs> {};
let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  jq = nixpkgs.jq;
  python_env = nixpkgs.python;
  rdkafka = (import ./rdkafka.nix {
    stdenv = nixpkgs.stdenv;
    fetchFromGitHub = nixpkgs.fetchFromGitHub;
    zlib = nixpkgs.zlib;
    perl = nixpkgs.perl;
    pkgconfig = nixpkgs.pkgconfig;
    python = python_env;
  });
  gcc = nixpkgs.gcc;
  libclang = nixpkgs.llvmPackages.libclang;
  kafka = nixpkgs.apacheKafka;
  zookeeper = nixpkgs.zookeeper;
  gdb = nixpkgs.gdb;
  java = nixpkgs.jre8;
  zlib = nixpkgs.zlib;
  rust = nixpkgs.latest.rustChannels.nightly.rust;
  pkgconfig = nixpkgs.pkgconfig;
in nixpkgs.stdenv.mkDerivation {

    name = "kafka-jq";
    src = ./.;

    # dependencies
    buildInputs = [ rust jq libclang gcc rdkafka kafka zookeeper gdb java zlib python_env pkgconfig ];

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
