# zubr_dsp
![dsp_zubr](dsp_zubr.png)
[![Rust CI](https://github.com/MalinkyZubr/zubr_dsp/actions/workflows/ci.yml/badge.svg)](https://github.com/MalinkyZubr/zubr_dsp/actions/workflows/ci.yml)

A highly modular signal processing pipeline designed for flexibility in parallel contexts.

## Features
- [x] General parallel pipeline structure, construction, running, management
- [x] Support for interleaved data, deconstruction and reconstruction of data
- [x] Support for feedback system architectures (needs testing)
- [x] Support for implementation of GPU bound computations
- [ ] Flexible control plane
- [ ] Optional TUI system for data visualization and control
- [ ] Support for recipes (reuse of commonly used workflows)
- [ ] Support for node aggregation (so many steps can run with no overhead on a single thread)
- [ ] DSP toolkit (RF and audio DSP computations, FFTs, modulation, filtering, etc.) Partially implemented, needs integration
- [ ] Data toolkit (codings, compression, stream encryption, etc). Partially implemented, needs integration
- [ ] Integration with SDR libraries
- [ ] Integrated tutorials
- [ ] benchmarking
- [ ] general optimizations, overhead reduction
- [ ] documentation (with diagrams and such)

## Statement of Purpose
zubr_dsp is a highly modular, code-first signal processing pipeline framework designed for deterministic, high-performance DSP development. Unlike runtime-validated frameworks, pipelines in zubr_dsp are statically constructed, with compile-time validation of node connectivity, data types, and overall graph topology. This ensures that invalid connections and mismatched types are caught at compile time, reducing runtime errors and increasing developer confidence.

Key architectural principles include:

Ownership-driven lifecycle: Each node enforces single-instance execution and safely manages its internal state, preventing race conditions and feedback loop deadlocks.

Dependency-based scheduling: Nodes execute only after all upstream dependencies are fulfilled, and bounded buffers propagate backpressure automatically.

Hybrid execution contexts: Nodes can operate in CPU-bound or asynchronous contexts, including GPU offload, without changing pipeline structure or introducing unsafe concurrency.

Batch-oriented computation: Nodes process arrays of samples rather than individual values, enabling efficient SIMD, GPU, and multithreaded execution.

Flexible pipeline composition: Developers declare operations and their order explicitly; the framework enforces correctness, while minimizing infrastructural complexity.

zubr_dsp is designed to give developers a “close to the metal” approach to DSP: pipelines are deterministic, type-safe, and extend naturally to heterogeneous compute resources. It is not a GUI-first tool like GNU Radio, but a code-native framework for rapid, reliable DSP pipeline construction with safety and performance guarantees baked in.

## Documentation
### WIP

## Usage
* Requries nightly release of Rust
* only tested on Ubuntu linux
* usage details are not provided yet since the project is still in very early stages of development. THIS IS NOT PRODUCTION READY and wont be for quite a while. But do keep your eyes peeled

## Testing
1. git clone https://github.com/MalinkyZubr/zubr_dsp.git
2. cd zubr_dsp
3. sudo apt-get update
   sudo apt-get install -y \
   pkg-config \
   libasound2-dev \
   libfontconfig1-dev \
   libfreetype6-dev \
   libexpat1-dev
4. rustup default nightly
5. cargo test

## Contribution

If anyone happens to find this and is interested, please make pull requests, create issues, everything.

## License
[MIT license](LICENSE)
