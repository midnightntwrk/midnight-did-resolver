{
  compactc,
  esbuild,
  buildNpmPackage,
  importNpmLock,
  nodejs_22,
  writeShellApplication,
  symlinkJoin,
}:

let
  bundle = buildNpmPackage {
    name = "midnight-did-serde-js";
    src = ../..;

    npmRoot = "./midnight-did-serde-js";
    npmDeps = importNpmLock { npmRoot = ../../midnight-did-serde-js; };
    inherit (importNpmLock) npmConfigHook;

    nativeBuildInputs = [
      compactc
      esbuild
    ];

    buildPhase = ''
      cd ./midnight-did-serde-js

      # run typecheck
      npm run build
      rm -rf dist

      # actual build
      compactc --skip-zk src/did.compact src/managed/did
      esbuild --bundle \
        --packages=external \
        --platform=node \
        --outdir=dist \
        --format=cjs \
        src/cli.ts
    '';

    installPhase = ''
      mkdir -p $out/dist
      mkdir -p $out/node_modules
      cp -r dist/* $out/dist
      cp -r node_modules/* $out/node_modules
    '';
  };
  wrapper = writeShellApplication {
    name = "midnight-did-serde-js";
    runtimeInputs = [ nodejs_22 ];
    text = ''
      export NODE_PATH=${bundle}/node_modules
      node ${bundle}/dist/cli.js "$@"
    '';
  };
in
symlinkJoin {
  name = "midnight-did-serde-js";
  paths = [
    bundle
    wrapper
  ];
}
