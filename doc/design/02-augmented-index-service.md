# Augmented Index Service

A crate index parallel to the crates.io-index that contains supplemental information, and the jobs
that maintain it.

## Scope
It is inevitable in the course of generating snapshots that some needed metadata will not be available in the standard crates.io-index. In order to perform efficient planning, a supplemental index will need to be generated and maintained.

### Goals
- Provide an additional set of metadata for crates.io crates
- Keep additional fields up to date
- Handle transfomations from crate sources to metadata

### Non Goals
- Modify the existing index
- Get any additional fields added to the index

## Overview

## Detailed Design

### Structure
The index will be structured identically to the crates.io-index. Most crates will be placed in
directory structures based on the first four letters of their name, as follows:

example_crate => /ex/am/example_crate.

In the case that the name is less than four letters long, it will go under a directory named
after the length of the name, as follows:

exa => /3/exa

The index will be persisted in a git repository identically to the crates.io-index. It will be
hosted within the cluster, but mirrored to github. It will stay separate from the project repo
so that it may be cloned by users.

### Contents
For now, the primary metadata element of interest is the dev dependency listing. Dev
dependencies can be found by inspection of the Cargo.toml file present at the root
of every `.crate` tarball.

The augmented index is not intended to be limited to just this singular field, however.
In practice it will be necessary to backfill additional fields, or update incorrect fields.

### Populating
To populate the field, an `ais-backfiller` job will be written thet performs the following steps:

1. Locate the augmented index, either by cloning it, or by reading a local directory.
2. Locate the original index, either by cloning it, or by reading a local directory.
3. Scan the existing augmented index and original index and join them into a combined index.
4. Identify entries that are either missing or invalid, and enqueue them to be backfilled.
5. For each crate to be backfilled, acquire the corresponding crate from LCS
6. Using the original index and the crate itself, generate the missing data and write it to the augmented index.
7. Commit and optionally push the augmented index to remote.
