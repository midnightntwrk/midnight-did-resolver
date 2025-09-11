{
  curl,
  dockerTools,
  neoprism-bin,
  tagSuffix ? "",
  version,
  extraPackages ? [ ],
}:

dockerTools.buildLayeredImage {
  name = "identus-neoprism";
  tag = "${version}${tagSuffix}";
  contents = [
    curl
    neoprism-bin
  ] ++ extraPackages;
  config = {
    Env = [
      "RUST_LOG=info,oura=warn"
      "NPRISM_ASSETS_PATH=/assets"
    ];
    Entrypoint = [ "/bin/neoprism-node" ];
    Cmd = [ ];
    WorkingDir = "/";
  };
}
