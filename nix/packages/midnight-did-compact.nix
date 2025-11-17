{
  stdenv,
  midnight-did-src,
  compactc,
}:

stdenv.mkDerivation {
  pname = "midnight-did-compact";
  version = "0.2.0-main";

  src = midnight-did-src;

  nativeBuildInputs = [ compactc ];

  buildPhase = ''
    runHook preBuild

    mkdir -p build/managed
    compactc $src/contract/src/did.compact build/managed/did

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out/src
    cp -r $src/contract/src/did.compact $out/src/did.compact
    cp -r build/managed $out/src/

    runHook postInstall
  '';
}
