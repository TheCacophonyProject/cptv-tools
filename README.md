# CPTV-Tools

CPTV stands for 'Cacophony Project TV', and is a bespoke file format for storing raw thermal video recordings created by our thermal camera software.

This repository contains a rust implementation of a CPTV decoder, with the aim of having one well-tested CPTV handling library which can be used by a variety of runtime environments:

- Web via WASM,
- Python via Py03 bindings
- Native code using vanilla Rust
