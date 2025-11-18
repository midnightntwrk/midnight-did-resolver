{
  buildNpmPackage,
  midnight-did-src,
  nodejs_22,
  version,
}:

buildNpmPackage {
  inherit version;
  pname = "midnight-did-js-node-modules";

  src = midnight-did-src;

  patches = [
    ./package-lock.patch
  ];

  nodejs = nodejs_22;

  npmDepsHash = "sha256-iFZHiBaXMNMAayCbS83veX2gkx8M2BgaS1Cpw3TxWaM=";

  dontNpmBuild = true;

  installPhase = ''
    runHook preInstall

    mkdir -p $out
    cp -r node_modules $out/
    cp -r api $out/
    cp -r cli $out/
    cp -r contract $out/
    cp -r did $out/
    cp -r domain $out/

    runHook postInstall
  '';
}
