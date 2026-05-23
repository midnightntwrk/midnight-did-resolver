import { defineConfig } from "vitepress";

const repoName = process.env.GITHUB_REPOSITORY?.split("/")[1] ?? "midnight-did-resolver";
const base = process.env.DOCS_BASE ?? (process.env.GITHUB_ACTIONS ? `/${repoName}/` : "/");

export default defineConfig({
  base,
  title: "Midnight DID Resolver",
  description: "Documentation for the Midnight DID resolver, manager, and local secret-storage packages.",
  cleanUrls: true,
  lastUpdated: true,
  markdown: {
    config(md) {
      const fence = md.renderer.rules.fence?.bind(md.renderer.rules);
      md.renderer.rules.fence = (tokens, idx, options, env, self) => {
        const token = tokens[idx];
        if (token.info.trim() === "mermaid") {
          return `<MermaidBlock encoded="${encodeURIComponent(token.content)}" />`;
        }
        return fence ? fence(tokens, idx, options, env, self) : self.renderToken(tokens, idx, options);
      };
    },
  },
  themeConfig: {
    logo: "/mark.svg",
    siteTitle: "Midnight DID Resolver",
    search: { provider: "local" },
    nav: [
      { text: "Guide", link: "/guide/" },
      { text: "Packages", link: "/packages/" },
      { text: "Services", link: "/services/" },
      { text: "Architecture", link: "/architecture/" },
      { text: "API Reference", link: "/api/" },
      { text: "Rust Resolver", link: "/rust/" },
      { text: "Source", link: "/source/" },
      { text: "GitHub", link: "https://github.com/midnightntwrk/midnight-did-resolver" },
    ],
    sidebar: {
      "/guide/": [
        {
          text: "Guide",
          items: [
            { text: "Overview", link: "/guide/" },
            { text: "Local Development", link: "/guide/local-development" },
            { text: "Resolver Getting Started", link: "/guide/getting-started-did-resolver" },
            { text: "Manager Getting Started", link: "/guide/getting-started-did-manager" },
            { text: "Testing Strategy", link: "/guide/testing-strategy" },
            { text: "Publishing", link: "/guide/publishing" },
          ],
        },
      ],
      "/packages/": [
        {
          text: "TypeScript Packages",
          items: [
            { text: "Overview", link: "/packages/" },
            { text: "Secret Storage", link: "/packages/secret-storage" },
            { text: "Secret Storage Examples", link: "/packages/secret-storage-examples" },
          ],
        },
      ],
      "/services/": [
        {
          text: "Services",
          items: [
            { text: "Overview", link: "/services/" },
            { text: "Resolver Service", link: "/services/did-resolver-service" },
            { text: "Extending Resolver", link: "/services/did-resolver-extension" },
            { text: "Manager Service", link: "/services/did-manager-service" },
            { text: "Wallet Setup", link: "/services/wallet-setup" },
            { text: "Secret Storage", link: "/services/secret-storage-workspace" },
            { text: "Sign & Verify", link: "/services/sign-verify-workspace" },
            { text: "DID Management", link: "/services/did-management-workspace" },
            { text: "Extending Manager", link: "/services/did-manager-extension" },
          ],
        },
      ],
      "/architecture/": [
        {
          text: "Architecture",
          items: [
            { text: "Overview", link: "/architecture/" },
            { text: "Manager Architecture", link: "/architecture/did-manager-service" },
            { text: "ADR: Service Split", link: "/architecture/adr-service-split" },
            { text: "ADR: Shared Seed and Profiles", link: "/architecture/adr-shared-seed-and-profiles" },
            { text: "ADR: HD Key Derivation", link: "/architecture/adr-hd-key-derivation-and-ledger-compatibility" },
          ],
        },
      ],
      "/api/": [
        {
          text: "API Reference",
          items: [
            { text: "Overview", link: "/api/" },
            { text: "Secret Storage", link: "/api/reference/secret-storage/" },
          ],
        },
      ],
      "/rust/": [
        {
          text: "Rust Resolver",
          items: [
            { text: "Overview", link: "/rust/" },
            { text: "Development Guide", link: "/rust/development-guide" },
            { text: "Design", link: "/rust/design" },
            { text: "Contract Deserialization", link: "/rust/contract-deserialization" },
            { text: "Integration Tests", link: "/rust/integration-tests" },
          ],
        },
      ],
      "/source/": [
        {
          text: "Source Documents",
          items: [
            { text: "Overview", link: "/source/" },
            { text: "Repository README", link: "/source/repository-overview" },
            { text: "Secret Storage README", link: "/source/secret-storage-readme" },
            { text: "Resolver Service README", link: "/source/did-resolver-service-readme" },
            { text: "Manager Service README", link: "/source/did-manager-service-readme" },
          ],
        },
      ],
    },
    socialLinks: [{ icon: "github", link: "https://github.com/midnightntwrk/midnight-did-resolver" }],
    footer: {
      message: "Midnight DID resolver and local identity services",
      copyright: "Apache-2.0",
    },
  },
});
