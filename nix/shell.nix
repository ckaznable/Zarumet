{
  mkShell,
  zarumet,
  rust-analyzer,
  rustup,
  cargo-nextest,
  cargo-about,
  alejandra,
}:
(mkShell.override {inherit (zarumet) stdenv;}) {
  inputsFrom = [zarumet];
  packages = [
    rust-analyzer
    rustup
    cargo-nextest
    cargo-about
    alejandra
  ];

  shellHook = ''
    read -p "Which shell do you use?: " shell

    $shell
    exit
  '';
}
