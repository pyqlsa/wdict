# wdict
Create dictionaries by scraping webpages.

## Take it for a spin
```bash
# build with nix and run the result
nix build .#
./result/bin/wdict --help

# just run it directly
nix run .# -- --help

# run it without cloning
nix run github:pyqlsa/wdict -- --help

# using a dev shell
nix develop .#
cargo build
./target/debug/wdict --help

# ...or a release version
cargo build --release
./target/release/wdict --help
```
