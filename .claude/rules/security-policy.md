# Security Policy

## Deny-Read
- .env, .env.*, secrets/**, credentials.*, *.pem, *.key
- node_modules/, target/, .git/objects/

## Deny-Edit
- Binary Files (*.wasm, *.so, *.dylib, *.exe)
- Generated Files (*.generated.*, *.pb.go)
- Lock Files (Cargo.lock, package-lock.json, yarn.lock)

## Deny-Run
- rm -rf (ohne expliziten Pfad)
- curl | sh, wget | bash
- sudo *
