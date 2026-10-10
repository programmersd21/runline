# development guide

## repo layout
- `src/main.rs`: entry point and terminal setup
- `src/app.rs`: state machine and update function
- `src/gh/`: github cli backend adapter and json parsers
- `src/ui/`: ratatui views and widgets
- `.github/workflows/`: ci on every push and pull request, release on a `v*` tag

## running tests
```bash
cargo test
```

## running demo mode
```bash
cargo run -- --demo
```

## cutting a release
```bash
# bump the version in Cargo.toml first, then:
cargo test
git tag v1.0.1 && git push origin v1.0.1
```
the release workflow refuses to publish a tag that does not match the
Cargo.toml version. re-run it without a new commit via the `workflow_dispatch`
trigger on the same tag.
