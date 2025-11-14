{
  stdenv,
  fetchzip,
  autoPatchelfHook,
  unzip,
  lib,
}:

let
  # Platform-specific configuration
  platformConfig = {
    x86_64-linux = {
      arch = "x86_64-unknown-linux-musl";
      sha256 = "sha256-WYuqZq3imdVYXkZu8ADOY7JB0Wn7I5Xecwq2Av27a8E=";
      needsPatchelf = true;
    };
    aarch64-darwin = {
      arch = "aarch64-darwin";
      sha256 = "sha256-vYaRebzDMTTDJBsOHf5xlkpeAIq7Nyl/UGVHBAWoqWs=";
      needsPatchelf = false;
    };
  };

  config = platformConfig.${stdenv.hostPlatform.system} or (throw "Unsupported platform: ${stdenv.hostPlatform.system}");
in
stdenv.mkDerivation rec {
  pname = "compactc";
  version = "0.26.0";

  src = fetchzip {
    url = "https://d3fazakqrumx6p.cloudfront.net/artifacts/compiler/compactc_v${version}/compactc_v${version}_${config.arch}.zip";
    sha256 = config.sha256;
    stripRoot = false;
  };

  nativeBuildInputs = [
    unzip
  ] ++ lib.optionals config.needsPatchelf [
    autoPatchelfHook
  ];

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin
    cp -r * $out/bin/
    chmod +x $out/bin/*

    runHook postInstall
  '';
}
