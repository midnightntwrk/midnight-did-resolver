{
  curl,
  dockerTools,
  midnight-resolver,
  tagSuffix ? "",
  version,
  extraPackages ? [ ],
}:

dockerTools.buildLayeredImage {
  name = "midnight-resolver";
  tag = "${version}${tagSuffix}";
  contents = [
    curl
    midnight-resolver
  ] ++ extraPackages;
  config = {
    Env = [
      "RUST_LOG=info,oura=warn"
    ];
    Entrypoint = [ "/bin/midnight-resolver" ];
    Cmd = [ ];
    WorkingDir = "/";
  };
}
