# wdict
Create dictionaries by scraping webpages or crawling local files.

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
Create dictionaries by scraping webpages or crawling local files.

Usage: wdict [OPTIONS] <--url <URL>|--theme <THEME>|--path <PATH>|--resume|--resume-strict>

Options:
  -u, --url <URL>
          URL to start crawling from

      --theme <THEME>
          Pre-canned theme URLs to start crawling from (for fun)

          Possible values:
          - star-wars:   Star Wars themed URL <https://www.starwars.com/databank>
          - tolkien:     Tolkien themed URL <https://www.quicksilver899.com/Tolkien/Tolkien_Dictionary.html>
          - witcher:     Witcher themed URL <https://witcher.fandom.com/wiki/Elder_Speech>
          - pokemon:     Pokemon themed URL <https://www.smogon.com>
          - bebop:       Cowboy Bebop themed URL <https://cowboybebop.fandom.com/wiki/Cowboy_Bebop>
          - greek:       Greek Mythology themed URL <https://www.theoi.com>
          - greco-roman: Greek and Roman Mythology themed URL <https://www.gutenberg.org/files/22381/22381-h/22381-h.htm>
          - lovecraft:   H.P. Lovecraft themed URL <https://www.hplovecraft.com>

  -p, --path <PATH>
          Local file path to start crawling from

      --resume
          Resume crawling from a previous run; state file must exist; existence of dictionary is optional; parameters from state are ignored, instead favoring arguments provided on the command line

      --resume-strict
          Resume crawling from a previous run; state file must exist; existence of dictionary is optional; 'strict' enforces that all arguments from the state file are observed

  -d, --depth <DEPTH>
          Limit the depth of crawling URLs

          [default: 1]

  -m, --min-word-length <MIN_WORD_LENGTH>
          Only save words greater than or equal to this value

          [default: 3]

  -x, --max-word-length <MAX_WORD_LENGTH>
          Only save words less than or equal to this value

          [default: 18446744073709551615]

  -j, --include-js
          Include javascript from <script> tags and URLs

  -c, --include-css
          Include CSS from <style> tags and URLs

      --filters <FILTERS>...
          Filter strategy for words; multiple can be specified (comma separated)

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
          - to-lower:     Transform words to lowercase
          - all-lower:    Ignore words that consist of all lowercase characters
          - any-lower:    Ignore words that contain any lowercase character
          - no-lower:     Ignore words that contain no lowercase characters
          - only-lower:   Keep only words that exclusively contain lowercase characters
          - to-upper:     Transform words to uppercase
          - all-upper:    Ignore words that consist of all uppercase characters
          - any-upper:    Ignore words that contain any uppercase character
          - no-upper:     Ignore words that contain no uppercase characters
          - only-upper:   Keep only words that exclusively contain uppercase characters
          - none:         Leave the word as-is

          [default: none]

  -s, --site-policy <SITE_POLICY>
          Site policy for discovered URLs

          Possible values:
          - same:      Allow crawling URL, only if the domain exactly matches
          - subdomain: Allow crawling URLs if they are the same domain or subdomains
          - sibling:   Allow crawling URLs if they are the same domain or a sibling
          - all:       Allow crawling all URLs, regardless of domain

          [default: same]

      --user-agent <USER_AGENT>
          User Agent string to send with requests

      --header <HEADER>
          HTTP headers to send with requests; can be specified multiple times (key=value)

  -r, --req-per-sec <REQ_PER_SEC>
          Number of requests to make per second

          [default: 10]

  -l, --limit-concurrent <LIMIT_CONCURRENT>
          Limit the number of concurrent requests to this value

          [default: 10]

  -o, --output <OUTPUT>
          File to write dictionary to (will be overwritten if it already exists)

          [default: wdict.txt]

      --append
          Append extracted words to an existing dictionary

      --no-write
          Skip writing words to an output file (i.e. save your disk while benchmarking)

      --output-state
          Write crawl state to a file

      --state-file <STATE_FILE>
          File to write state, json formatted (will be overwritten if it already exists)

          [default: state-wdict.json]

  -v, --verbose...
          Increase logging verbosity

  -q, --quiet...
          Decrease logging verbosity

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

```
<!-- readme-help end -->

## Lib
This crate exposes a library, but for the time being, the interfaces should be considered unstable.

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

