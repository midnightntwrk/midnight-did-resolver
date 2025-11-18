{
  stdenv,
  midnight-did-src,
  compactc,
  nodejs_22,
  typescript,
  midnight-circuit-params,
}:

stdenv.mkDerivation {
  pname = "midnight-did-js";
  version = "0.2.0-main";

  src = midnight-did-src;

  nativeBuildInputs = [
    compactc
    nodejs_22
    typescript
  ];

  buildPhase = ''
    runHook preBuild

    export HOME=$TMPDIR

    # compactc looks for parameters in $HOME/.cache/midnight/zk-params/
    mkdir -p $HOME/.cache/midnight/zk-params
    cp -r ${midnight-circuit-params}/* $HOME/.cache/midnight/zk-params/
    mkdir -p contract/src/managed
    compactc contract/src/did.compact contract/src/managed/did

    # build contract dist
    npm run build -w contract

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out/src
    cp -r contract/src/did.compact $out/src/did.compact
    cp -r contract/src/managed $out/src/

    runHook postInstall
  '';
}
