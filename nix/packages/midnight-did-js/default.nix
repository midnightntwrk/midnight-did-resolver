{
  stdenv,
  midnight-did-src,
  compactc,
  nodejs_22,
  typescript,
  midnight-circuit-params,
  callPackage,
}:

let
  version = "0.1.0";
  nodeModules = callPackage ./node-modules.nix { inherit version; };
in
stdenv.mkDerivation {
  inherit version;
  pname = "midnight-did-js";

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

    cp -r ${nodeModules}/node_modules ./
    npm run build -w contract
    npm run build -w domain
    npm run build -w did
    npm run build -w api
    npm run build -w cli

    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp package.json $out/package.json
    cp -r node_modules $out/
    cp -r api $out/
    cp -r cli $out/
    cp -r contract $out/
    cp -r did $out/
    cp -r domain $out/

    runHook postInstall
  '';
}
