{
  buildNpmPackage,
  midnight-did-src,
  nodejs_22,
}:

buildNpmPackage {
  pname = "midnight-did-node-modules";
  version = "0.1.0";

  src = midnight-did-src;

  patches = [
    ./patches/midnight-did-package-lock.patch
  ];

  nodejs = nodejs_22;

  npmDepsHash = "sha256-iFZHiBaXMNMAayCbS83veX2gkx8M2BgaS1Cpw3TxWaM=";

  dontNpmBuild = true;

  installPhase = ''
    runHook preInstall

    export HOME=$TMPDIR

    mkdir -p $out
    cp -r node_modules $out/

    runHook postInstall
  '';
}
