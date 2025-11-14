{
  stdenv,
  fetchzip,
  autoPatchelfHook,
  unzip,
}:

stdenv.mkDerivation rec {
  pname = "compactc";
  version = "0.26.0";

  src = fetchzip {
    url = "https://d3fazakqrumx6p.cloudfront.net/artifacts/compiler/compactc_v${version}/compactc_v${version}_x86_64-unknown-linux-musl.zip";
    sha256 = "sha256-WYuqZq3imdVYXkZu8ADOY7JB0Wn7I5Xecwq2Av27a8E=";
    stripRoot = false;
  };

  nativeBuildInputs = [
    autoPatchelfHook
    unzip
  ];

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin
    cp -r * $out/bin/
    chmod +x $out/bin/*

    runHook postInstall
  '';
}
