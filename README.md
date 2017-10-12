# Stockpile

An alternative resolution strategy for Cargo crates.

See [Stackage](https://www.stackage.org/) to learn what its all about.

## Idiosyncratic development practices

### Rust
- Indent 2 spaces, not 4
- Name internal crates using the directory structure, such as: tools/cli -> "tools-cli"
- Do not bundle imports from the same package onto one line: instead, fully qualify each import.

### General
- Do not bundle configuration and code changes into the same PR
