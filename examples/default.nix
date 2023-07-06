with import <nixpkgs> {};

rustPlatform.buildRustPackage rec {
  pname = "marswm";
  version = "0.5.0";

  src = fetchFromGitHub {
    owner = "jzbor";
    repo = pname;
    rev = "146501f455cd28165f1c5514f7b1a3f30fc128af";
    sha256 = "sha256-1hV7lDTmHDrE7giiYRwegb228kNBU1HMAJtgX3SYoyo=";
  };

  cargoLock = {
    lockFile = "${src}/Cargo.lock";
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    xmenu
    xorg.libX11
    xorg.libXft
    xorg.libXinerama
    xorg.libXrandr
  ];

  meta = with lib; {
    description = "A modern window manager featuring dynamic tiling (rusty successor to moonwm).";
    homepage = "https://github.com/jzbor/marswm";
    license = licenses.mit;
    maintainers = [ maintainers.jzbor ];
  };
}

