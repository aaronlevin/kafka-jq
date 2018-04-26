with import <nixpkgs> {};
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
  kafka = nixpkgs.apacheKafka;
  zookeeper = nixpkgs.zookeeper;
  gdb = nixpkgs.gdb;
  java = nixpkgs.jre8;
  zlib = nixpkgs.zlib;
  rust = nixpkgs.latest.rustChannels.nightly.rust;
  binutils = nixpkgs.binutils;
  pkgconfig = nixpkgs.pkgconfig;
  libclang = nixpkgs.llvmPackages.libclang;
in
  llvmPackages.stdenv.mkDerivation rec {
    name = "kafka-jq-env";
    buildInputs = [
      libclang rust jq gcc rdkafka kafka zookeeper gdb java zlib python_env binutils pkgconfig
    ];
    shellHook = "export LIBCLANG_PATH=${libclang}/lib";
  }
