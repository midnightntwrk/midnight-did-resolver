{ linkFarm, fetchurl }:

let
  baseUrl = "https://midnight-s3-fileshare-dev-eu-west-1.s3.eu-west-1.amazonaws.com";
  makeCircuitParam = name: sha256: {
    inherit name;
    path = fetchurl {
      url = "${baseUrl}/${name}";
      inherit sha256;
    };
  };
  circuitParams = [
    {
      name = "bls_filecoin_2p1";
      sha256 = "0yzjwwl2jj8wy17ndriq6hiq8lv3701vchdx0b35h1scr6r6bjf7";
    }
    {
      name = "bls_filecoin_2p2";
      sha256 = "1v6620aqyhc22ir42smysydnm06p6x9bwicp4rj64vy0j5afbahw";
    }
    {
      name = "bls_filecoin_2p3";
      sha256 = "021455sbqi3qdgmv2x8i2bha1idvzgy951af6vl9jf3sd95qfh1w";
    }
    {
      name = "bls_filecoin_2p4";
      sha256 = "0jpycazjl0bpd7kklyadi03smahs95y4dncfgah2j13qbl6nhnki";
    }
    {
      name = "bls_filecoin_2p5";
      sha256 = "1s2yjvnpd4lbxcmhlhhqybps388pkbjinc0qh9591h4zjjqzf50v";
    }
    {
      name = "bls_filecoin_2p6";
      sha256 = "0fir3zgy5d4nlahpdbaafq0mhq6yw1kpd1zskl1f71592qbjsq1c";
    }
    {
      name = "bls_filecoin_2p7";
      sha256 = "1xsd9isj3vf9i5pb6lzw0xm815bh345wzrgr5n1gfnqvhajfwzga";
    }
    {
      name = "bls_filecoin_2p8";
      sha256 = "1q7gfgwa7nnnzvq5qs00gzc58v0bg8lx0q9f3kbizjnq7ndpkik2";
    }
    {
      name = "bls_filecoin_2p9";
      sha256 = "04491lafjisnv8zr66p2z01x1jd1pwsb1wakqfjd21x57dry9hsx";
    }
    {
      name = "bls_filecoin_2p10";
      sha256 = "0aNAPB+Gaegu0o2TkeEwEa6naAGyj+FLQr920UG076I=";
    }
    {
      name = "bls_filecoin_2p11";
      sha256 = "tQR/BYANvYT9HqQ7lqiFDhKLelle0TLNcliMwssUayk=";
    }
    {
      name = "bls_filecoin_2p12";
      sha256 = "syeRd1r1//GuXq1oLD2IMpF+uwZStDz4EKHjlW6yenE=";
    }
    {
      name = "bls_filecoin_2p13";
      sha256 = "ua9DiSw8uQMh+gCjbl5ZBR81bfFF1/WDaFMfKNISk3s=";
    }
    {
      name = "bls_filecoin_2p14";
      sha256 = "SSPlp/u3Fdgc21wDucDiEXaNNczFLYL0nD2TvPjTalY=";
    }
    {
      name = "bls_filecoin_2p15";
      sha256 = "15wwwff7mx43b9ysjc8qvhwpp6847h0kgv4m47h056qbyw6aqbqn";
    }
    {
      name = "bls_filecoin_2p16";
      sha256 = "0wzrfcssvjqfysrrgk1rzq6z0xi1pqbk4mm6gjdmwr76gw3hvg2f";
    }
  ];
in
linkFarm "midnight-circuit-params" (map (p: makeCircuitParam p.name p.sha256) circuitParams)
