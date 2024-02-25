# wdict
Create dictionaries by scraping webpages.

Similar tools (some features inspired by them):
- [CeWL](https://github.com/digininja/CeWL)
- [CeWLeR](https://github.com/roys/cewler)

## Take it for a spin
```bash
# build with nix and run the result
nix build .#
./result/bin/wdict --help

# just run it directly
nix run .# -- --help

# run it without cloning
nix run github:pyqlsa/wdict -- --help

# install from crates.io
# (nixOS users may need to do this within a dev shell)
cargo install wdict

# using a dev shell
nix develop .#
cargo build
./target/debug/wdict --help

# ...or a release version
cargo build --release
./target/release/wdict --help
```
## Usage
<!-- readme-help -->
```bash
Create dictionaries by scraping webpages.

Usage: wdict [OPTIONS] <--url <URL>|--theme <THEME>>

Options:
  -u, --url <URL>
          URL to start crawling from

      --theme <THEME>
          Pre-canned theme URLs to start crawling from (for fun, demoing features, and sparking new ideas)

          Possible values:
          - star-wars: Star Wars themed URL <https://www.starwars.com/databank>
          - tolkien:   Tolkien themed URL <https://www.quicksilver899.com/Tolkien/Tolkien_Dictionary.html>
          - witcher:   Witcher themed URL <https://witcher.fandom.com/wiki/Elder_Speech>
          - pokemon:   Pokemon themed URL <https://www.smogon.com>
          - bebop:     Cowboy Bebop themed URL <https://cowboybebop.fandom.com/wiki/Cowboy_Bebop>

  -d, --depth <DEPTH>
          Limit the depth of crawling links

          [default: 1]

  -m, --min-word-length <MIN_WORD_LENGTH>
          Only save words greater than or equal to this value

          [default: 3]

  -r, --req-per-sec <REQ_PER_SEC>
          Number of requests to make per second

          [default: 20]

  -o, --output <OUTPUT>
          File to write dictionary to (will be overwritten if it already exists)

          [default: wdict.txt]

      --filters <FILTERS>...
          Filter strategy for words; multiple can be specified (comma separated)

          [default: none]

          Possible values:
          - deunicode:    Transform unicode according to <https://github.com/kornelski/deunicode>
          - decancer:     Transform unicode according to <https://github.com/null8626/decancer>
          - all-numbers:  Ignore words that consist of all numbers
          - any-numbers:  Ignore words that contain any number
          - no-numbers:   Ignore words that contain no numbers
          - only-numbers: Keep only words that exclusively contain numbers
          - all-ascii:    Ignore words that consist of all ascii characters
          - any-ascii:    Ignore words that contain any ascii character
          - no-ascii:     Ignore words that contain no ascii characters
          - only-ascii:   Keep only words that exclusively contain ascii characters
          - none:         Leave the word as-is

  -j, --inclue-js
          Include javascript from <script> tags and links

  -c, --inclue-css
          Include CSS from <style> tags links

      --site-policy <SITE_POLICY>
          Site policy for discovered links

          [default: same]

          Possible values:
          - same:      Allow crawling links, only if the domain exactly matches
          - subdomain: Allow crawling links if they are the same domain or subdomains
          - all:       Allow crawling all links, regardless of domain

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

```
<!-- readme-help end -->

## Lib
This crate exposes a library, but for the time being, the interfaces should be considered unstable.

## TODO
A list of ideas for future work:
 - archive mode to crawl and save pages locally
 - build dictionaries from local (archived) pages
 - support different mime types
 - more word filtering options
 - better async

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

