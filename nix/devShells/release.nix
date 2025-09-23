{ pkgs }:

let
  rootDir = "$ROOT_DIR";
  scripts = {
    releaseMidnightDidResolverImage = pkgs.writeShellApplication {
      name = "releaseMidnightDidResolverImage";
      runtimeInputs = with pkgs; [
        nix
        docker
        coreutils
        git
      ];
      text = ''
        cd "${rootDir}"
        TAG=0.1.0-alpha.1
        echo "Building midnight-did-resolver-docker-linux-amd64..."
        nix build .#midnight-did-resolver-docker-amd64 -o result-amd64
        echo "Building midnight-did-resolver-docker-linux-arm64..."
        nix build .#midnight-did-resolver-docker-arm64 -o result-arm64

        echo "Loading images into Docker..."
        docker load < ./result-amd64
        docker load < ./result-arm64

        echo "Tagging images..."
        docker tag midnight-did-resolver:0.1.0-amd64 "patextreme/midnight-did-resolver:$TAG-amd64"
        docker tag midnight-did-resolver:0.1.0-arm64 "patextreme/midnight-did-resolver:$TAG-arm64"

        echo "Cleaning up build artifacts..."
        rm -rf ./result-amd64
        rm -rf ./result-arm64

        echo "Pushing images to Docker Hub..."
        docker push "patextreme/midnight-did-resolver:$TAG-amd64"
        docker push "patextreme/midnight-did-resolver:$TAG-arm64"

        echo "Creating and pushing multi-arch manifest..."
        docker manifest create "patextreme/midnight-did-resolver:$TAG" \
          "patextreme/midnight-did-resolver:$TAG-amd64" \
          "patextreme/midnight-did-resolver:$TAG-arm64"
        docker manifest push "patextreme/midnight-did-resolver:$TAG"

        echo "Release complete!"
      '';
    };
  };
in
pkgs.mkShell {
  packages = builtins.attrValues scripts;

  shellHook = ''
    export ROOT_DIR=$(${pkgs.git}/bin/git rev-parse --show-toplevel)
    ${pkgs.cowsay}/bin/cowsay "Release shell ready at project root: ${rootDir}"
    cd "${rootDir}"
  '';
}
