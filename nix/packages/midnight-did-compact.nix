{
  midnight-did-src,
  runCommand,
  compactc,
}:

runCommand "midnight-did-compact"
  {
    src = midnight-did-src;
  }
  ''
    mkdir -p $out/src
    cp -r $src/contract/src/did.compact $out/src/did.compact
    ${compactc}/bin/compactc --skip-zk $src/contract/src/did.compact $out/src/managed/did
  ''
