# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Added

- `Transport` implementations for `&T`, `Box<T>`, `Rc<T>` and `Arc<T>`, which also covers `Box<dyn Transport + Send + Sync>` and `Arc<dyn Transport>`
- `Debug` implementations for metrics, clients and transports, and `Clone` for metrics and `ClientBuilder`

### Changed

- Tags passed to `GaugeHandle::*_with_tags` no longer need to outlive the client borrow
- `Timer` now requires `T: Transport` in its definition

### Fixed

- `Timer` is now sent when dropped, as documented, instead of being silently discarded
- `UdpTransport::connect` now works with IPv6 addresses

## 0.2.1 - 2026-05-31

### Added

- `Transport` implementation for `Box<dyn Transport>`

## 0.2.0 - 2026-05-31

### Changed

- Introduction of `Transport` trait to make `Client` generic over sending protocol [#1](https://github.com/zlikavac32/metco-client/pull/1)

## 0.1.0 - 2026-05-31

- First release
