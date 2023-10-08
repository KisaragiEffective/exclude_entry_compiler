# exclude_entry_compiler
A tiny data-format converter for personal use (supports uBO and uBL)

## Input

every input must be defined as JSON file.

example:

```json
[
{
"type": "domain",
"match": "literal",
"domain": "some-bad-domain.local"
},
{
"type": "path",
"match": "literal",
"path": "https://some-domain.local/bad"
}
]
```

## Command line

* `-i`: input. Specify path to a file. See above.
* `-o`: output. Specify path to a file.
* `-h`: header. May specify zero or more times. Header is shown as comments, therefore it will not affect listing.
* `--target` : target.
  * `uBlacklist`: create list for uBlacklist.
  * `uBlockOrigin`: create list for uBlockOrigin.
* `--feature-flag`: feature flag.
  * `Base`: base.
  * `GoogleSearchPrefix`: includes google search.
