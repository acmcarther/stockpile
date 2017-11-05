# Local Crate Service (LCS)

The name of both a collection of local crate blobs, and the software used to manage them.

## Scope

While building out a snapshot-based Rust crate resolution system, additional analyses will need
to be made against the global crate repository. Although some of the information will be available
in the existing crates.io crate index, some information will not be. One motivating example is
the set of development dependencies. This information is not currently available in the existing
index, so each crates' own Cargo.toml will need to be referenced

### Goals

- Locally vendor all crates from crates.io
- Provide a low volume store for internal jobs
- Maintain freshness of 24h or better

### Non Goals

- Serve crate blobs in lieu of Crates.io
- Provide a venue for modified versions of crates
- Act as a private crate repository

## Overview

This document prescribes simple S3-like storage system, and a regularly scheduled cron task which
regularly pulls crates that are missing locally.

## Detailed Design

### Repository Maintenance: lcs-fetcher

A cron-like fetcher job will be built (lcs-fetcher), with the following high level
lifecycle:

1. Pull crates.io-index to local storage /tmp
2. For each crate in index, idenfify if crate is present in storage
3. For each missing crate, pull locally, and publish to storage

Configuration options include:
- max_session_crates: Uint = None: Total number of crates to download in a single application session

Monitoring includes:
- index_crate_version_count { crate_name } = gauge: A gauge indicating the number of observed
versions per crate
- scanned_crates = counter: A counter incremented as crates are identified
- downloaded_crates = counter: A counter incremented as crates are downloaded

### Infrastructure

All jobs and storage will run on acmcarther@'s local cluster.

Monitoring and Alerting
- Monitoring will be provided by a cluster local prometheus instance.
- Alerting will be configured to alert on high or low volumes of scanned crates

Storage
- Crates will be stored persistently in Minio through the S3 API
- lcs.metadata will be stored in cluster Etcd, or Minio directly if too large for Etcd

Job Scheduling
- lcs-fetcher will be scheduled by a CronJob task in the cluster at some regular frequency.

## Security and Privacy

For the short term, access to the file store directly through the S3 API will be available. No
immediate issues are forseen with this strategy, as the crates themselves are not intended for
public consumption.
