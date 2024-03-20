# BanyanFS

BanyanFS is a work-in-progress distributed file system that is designed to be
privacy centric, with collaborative change tracking and versioning, that can
scale to exabytes of data.

This library is currently powering the file management system in the
[Banyan](https:://banyan.computer) storage platform.

## Features

- [x] End-to-end encryption
- [x] Block-based network storage
- [x] Local block cacheing
- [x] Native CID addressing for internal files and external encrypted blocks
- [x] Encryption key management
- [x] Full WASM support
- [x] Async & Multithread support
- [ ] Recursive mounting
- [ ] Merkle-CDRT based version merging
- [ ] Native IPFS integration
- [ ] FUSE filsystem
- [ ] S3 service

## Contributing

We're currently in the process of cleaning up some cruft, standardizing some
internal patterns, and formalizing a specification for both the transfer
protocol and the behavior of the internal file system that has been in progress
alongside the development of the Banyan platform.

We welcome issues and general feedback. If you're interested in contributing a
fix you're welcome to open a PR, please open an issue before pursuing any
refactors or feature changes so we can make sure they're in line with our clean
up efforts.

## Development

If you would like to use BanyanFS in your own project, we maintain protocol and
format compatibility between major versions but currently do not backfill fixes
or maintain LTS support for any specific version. We recommend using the latest
version for any new projects. If you do encounter any issues or have
compatibility concerns please open an issue and we'll do our best to address
it.
