# Guide

Start here when you are working on resolver, manager, or secret-storage code.

## Five-minute Docker demo

The fastest way to explore the components is the local Docker Compose demo:

```bash
just demo
```

It runs the resolver and Midnight DID Manager demo on localhost with local
Midnight infrastructure. The manager is not a production wallet or public DID
control plane: DID CRUD flows are illustrative, the secret-store passphrase is
required again after restart, and recovery is not implemented in v0.1.0.

- [Local Development](/guide/local-development)
- [Resolver Getting Started](/guide/getting-started-did-resolver)
- [Manager Getting Started](/guide/getting-started-did-manager)
- [Testing Strategy](/guide/testing-strategy)
- [Publishing](/guide/publishing)
