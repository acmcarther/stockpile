# (WIP) Snapshot Service

The collection of canonical crate version snapshots, and the planner jobs that create and verify them.

## Scope

### Goals

### Non Goals

## Overview

## Detailed Design

### Snapshot Properties

### Service Configuration

### Snapshot Resolution

## Security and Privacy

### Repository

As a matter of course, the repository will only provide write access to the service itself. As a second additional check, the contents of the snapshots themselves can be checksummed and included in the project configuration (detailed in CLI usage).

### Service

Raw inputs for the resoltuion service take the form of the crates.io-index, the augmented-index, and the local-crate-service. The augmented index itself is entirely self contained within stockpile (generated and used only internally). The crates.io-index is provided via external services, but may be mirrored if integrity is a concern. Finally, crates present in the local-crate-service provide the standard guarantees -- that is to say that they are unaudited code that must be built and run in an isolated environment.
