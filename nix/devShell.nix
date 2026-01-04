{ pkgs, lib, ... }:
let
  nix-tools = with pkgs; [
    nil
    nixd
    nixfmt-rfc-style
  ];
  wasm-tools = with pkgs; [
    wasm-bindgen-cli
    wasm-pack
  ];
  build-tools = with pkgs; [
    pkg-config
    autoconf
    automake
    cmake
  ];
  extraPackages =
    with pkgs;
    [
      libclang
      webrtc-audio-processing
      libopus
    ]
    ++ lib.optionals stdenv.isLinux [
      pkgs.libtool
      pkgs.alsa-lib.dev
    ]
    ++ lib.optionals stdenv.isDarwin [
      pkgs.glibtool
    ];
in
{
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [
      "rustc"
      "rust-src"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
    ];
    targets = [
      "wasm32-unknown-unknown"
      "x86_64-unknown-linux-gnu"
      "aarch64-apple-darwin"
    ];
  };

  languages.javascript = {
    enable = true;
    npm.enable = true;
    yarn.enable = true;
  };

  packages = wasm-tools ++ build-tools ++ nix-tools ++ extraPackages;

  env.LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

  scripts = {
    build-release = {
      exec = ''
        ${pkgs.cargo}/bin/cargo build --release
      '';
      };
    run-release = {
      exec = ''
        ${pkgs.cargo}/bin/cargo run --bin=hacat --release -- "$@"
      '';
      };
    };
}
