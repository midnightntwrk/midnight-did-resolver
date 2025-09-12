{
  curl,
  dockerTools,
  midnight-did-resolver,
  tagSuffix ? "",
  version,
  extraPackages ? [ ],
}:

dockerTools.buildLayeredImage {
  name = "midnight-did-resolver";
  tag = "${version}${tagSuffix}";
  contents = [
    curl
    midnight-did-resolver
  ] ++ extraPackages;
  config = {
    Env = [
      "RUST_LOG=info,oura=warn"
    ];
    Entrypoint = [ "/bin/midnight-did-resolver" ];
    Cmd = [ ];
    WorkingDir = "/";
  };
}
