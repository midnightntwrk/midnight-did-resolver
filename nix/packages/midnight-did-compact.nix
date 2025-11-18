{
  stdenv,
  midnight-did-src,
  compactc,
  midnight-circuit-params,
}:

stdenv.mkDerivation {
  pname = "midnight-did-compact";
  version = "0.2.0-main";

  src = midnight-did-src;

  nativeBuildInputs = [ compactc ];

  buildPhase = ''
    runHook preBuild

    export HOME=$TMPDIR

    # compactc looks for parameters in $HOME/.cache/midnight/zk-params/
    mkdir -p $HOME/.cache/midnight/zk-params
    cp -r ${midnight-circuit-params}/* $HOME/.cache/midnight/zk-params/
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
