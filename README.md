# logi-battery

Logitech 设备电池读取库（FFI 友好，提供 `cdylib` / `staticlib`）。

本项目基于 [OpenLogi](https://github.com/OpenLogi/OpenLogi) 的部分 crate（`openlogi-core`、`openlogi-hidpp`、`openlogi-hid`、`openlogi-device` 等）裁剪而来，聚焦电池状态读取能力，并新增 FFI 层。

## 开源协议（License Notice）

- 上游项目 OpenLogi 采用 **MIT OR Apache-2.0** 双许可。本仓库遵守其协议要求，保留同等双许可：
  - [LICENSE-MIT](LICENSE-MIT)
  - [LICENSE-APACHE](LICENSE-APACHE)
  - 本仓库整体以 `MIT OR Apache-2.0` 分发，原始版权声明保留。
- `crates/openlogi-hidpp` 是 [`hidpp`](https://crates.io/crates/hidpp) 协议 crate 的硬分支，原 crate 采用 **0BSD** 许可，其声明见 [crates/openlogi-hidpp/LICENSE](crates/openlogi-hidpp/LICENSE)。
- 感谢上游 OpenLogi 及 `hidpp` 项目作者的贡献。

## 构建

```sh
cargo build --release
```

产物位于 `target/release/`（`logi_battery.dll` / `.so` / `.a`），FFI 接口见 [docs/ffi.md](docs/ffi.md)。
