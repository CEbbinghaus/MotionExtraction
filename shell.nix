let
	rust-overlay = (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz"));
in
{ pkgs ? import <nixpkgs> { overlays = [ rust-overlay ]; } }:
pkgs.mkShellNoCC {
	# Kanidm dependencies
	buildInputs = with pkgs; [
		pkg-config
		
		(rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)

		clang
		llvmPackages.bintools
	];
	
	# https://github.com/rust-lang/rust-bindgen#environment-variables
	LIBCLANG_PATH = pkgs.lib.makeLibraryPath [ pkgs.llvmPackages_latest.libclang.lib ];
}
