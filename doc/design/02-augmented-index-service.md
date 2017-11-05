# Augmented Index Service

A crate index parallel to the crates.io-index that contains supplemental information, and the jobs
that maintain it.

## Scope



### Goals
- Provide an additional set of metadata for crates.io crates
- Keep additional fields up to date
- Handle transfomations from crate sources to metadata

### Non Goals
- Modify the existing index
- Get any additional fields added to the index

## Overview

## Detailed Design

### Index
The index will be structured identically to the crates.io-index. Most crates will be placed in
directory structures based on the first four letters of their name, as follows:

example_crate => /ex/am/example_crate.

In the case that the name is less than four letters long, it will go under a directory named
after the length of the name, as follows:

exa => /3/exa

The index will be persisted in a git repository identically to the crates.io-index. It will be
hosted within the cluster, but mirrored to github. It will stay separate from the project repo
so that it may be cloned by users.
