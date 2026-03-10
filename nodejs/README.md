* This is vibe-coded poc that works only on arm mac
* `sf_core_client` need a decent refactor and migration to napi-rs

```
# 1. Build Rust core (from repo root)
CARGO_TARGET_DIR=target cargo build --package sf_core

# 2. Build and run Node.js driver
npm install         # installs deps + compiles C++ addon
npm run build       # recompile C++ addon (if changed)
npm run demo        # run the demo (needs parameters.json at repo root)
npm run generate    # generate TypeScript client for the sf_core
```
