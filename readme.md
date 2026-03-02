# Rust-template

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
Software such as GNU radio is an excellent resource for developing prototype and production RF systems. They provide excellent user experience and simple UI for rapid development. On the other end, there are highly optimized, specialized pieces of software for certain applications, like cellular. With the already rich ecosystem of DSP software in mind, what reason does zubr_dsp have to exist (other than being fun to muck around with and develop)
1. GNURadio is generally GUI-driven, GUI-first development. Blocks are dragged and dropped into the environment to construct pipelines. zubr_dsp is code-first gui-optional. Pipelines are constructed by attaching modules together in Rust code
2. GNURadio uses C++ with python as a control plane abstraction. This is very convenient for development, but for zubr_dsp I tried a different approach. DSP and control plane are all driven by Rust. Pipeline construction is driven by Rust's type inference system, and compile time checks

Overall, the design philosophy aims to reduce the distance between DSP implementation and behavior, while still abstracting away unecessary infrastructural details. The most a developer should need to know to use this software is:
* what operations do they need to run and in what order
* what are the input and output datatypes of these mathematical operations

And from this rapidly implement low level DSP pipelines.

This is by no means a replacement to gnuradio. But eventually, a tool people can use if they want to take a closer to the ground approach to DSP development.

## Documentation
### WIP

## Usage

* usage details are not provided yet since the project is still in very early stages of development. THIS IS NOT PRODUCTION READY and wont be for quite a while. But do keep your eyes peeled

## Testing

From the project root directory, just run cargo --test. For now, this is the only running feature of the project, to verify marked items in the feature list.

## Contribution

If anyone happens to find this and is interested, please make pull requests, create issues, everything.

## License
[MIT license](LICENSE)
