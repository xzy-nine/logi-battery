//! Logitech 设备电量读取库
//!
//! 从 OpenLogi 提取，直接复用其 HID++ 协议层（`openlogi-hidpp`）、
//! 设备层（`openlogi-device`）与主机后端（`openlogi-hid`）。

/// 重新导出 openlogi-hid 的全部公开 API
/// （含其转发的 openlogi-device / hidpp 类型）
pub use openlogi_hid::*;

/// openlogi-core 的类型（DeviceInventory、PairedDevice、BatteryInfo 等）
pub use openlogi_core::*;

/// C FFI 导出（lb_* 函数）
pub mod ffi;
