# Contributing

This repository is public for visibility and CI efficiency, but it is currently maintained as a solo prototype.

## Contribution policy

Only `@cirvine-MSFT` is authorized for direct repository contributions. Outside pull requests are not accepted for now and may be closed automatically.

If you have feedback, open an issue with the context needed to evaluate it. Please do not spend time preparing a pull request unless the maintainer explicitly asks for one.

## Security issues

Do not report vulnerabilities in public issues or pull requests. Follow [SECURITY.md](SECURITY.md) instead.

## Maintainer checks

Before merging maintainer-authored changes, run the relevant checks:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
.\target\release\game-of-life.exe
```
