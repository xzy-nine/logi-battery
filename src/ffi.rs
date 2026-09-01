//! C FFI 导出层。
//!
//! 直接复用 `openlogi-hid` 的完整 HID++ 实现（枚举 + 电量探测），
//! 通过 `lb_enumerate_devices` 一次调用即可获得所有 Logitech 设备的电量信息。

use std::cell::RefCell;

use openlogi_core::device::{BatteryLevel, BatteryStatus};
use openlogi_hid::enumerate;

/// C 兼容的设备信息（含电量）
#[repr(C)]
pub struct LbDeviceInfo {
    /// USB Vendor ID（0x046D = Logitech）
    pub vendor_id: u16,
    /// USB Product ID
    pub product_id: u16,
    /// 设备名称（NUL 结尾 UTF-8）
    pub name: [u8; 256],
    /// 接收器配对槽位（1..=6；直连设备为 0）
    pub slot: u8,
    /// 是否在线（1 = 在线）
    pub online: u8,
    /// 是否有电量数据（1 = 有）
    pub has_battery: u8,
    /// 电量百分比 0-100
    pub percentage: u8,
    /// 电量等级：0=Critical 1=Low 2=Good 3=Full 4=Unknown
    pub level: u8,
    /// 充电状态：0=Discharging 1=Charging 2=ChargingSlow 3=Full 4=Error 5=Unknown
    pub status: u8,
}

/// C 兼容的设备列表
#[repr(C)]
pub struct LbDeviceList {
    /// 设备数组（由 `lb_enumerate_devices` 分配，须用 `lb_free_devices` 释放）
    pub devices: *mut LbDeviceInfo,
    /// 设备数量
    pub count: i32,
}

/// C 兼容的电池信息（用于 `lb_read_battery`）
#[repr(C)]
pub struct LbBatteryInfo {
    /// 电量百分比 0-100
    pub percentage: u8,
    /// 电量等级：0=Critical 1=Low 2=Good 3=Full 4=Unknown
    pub level: u8,
    /// 充电状态：0=Discharging 1=Charging 2=ChargingSlow 3=Full 4=Error 5=Unknown
    pub status: u8,
}

thread_local! {
    static LAST_ERROR: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn set_last_error(msg: &str) {
    LAST_ERROR.with(|e| {
        let mut e = e.borrow_mut();
        e.clear();
        e.extend_from_slice(msg.as_bytes());
        e.push(0);
    });
}

fn level_to_u8(level: BatteryLevel) -> u8 {
    match level {
        BatteryLevel::Critical => 0,
        BatteryLevel::Low => 1,
        BatteryLevel::Good => 2,
        BatteryLevel::Full => 3,
        BatteryLevel::Unknown => 4,
    }
}

fn status_to_u8(status: BatteryStatus) -> u8 {
    match status {
        BatteryStatus::Discharging => 0,
        BatteryStatus::Charging => 1,
        BatteryStatus::ChargingSlow => 2,
        BatteryStatus::Full => 3,
        BatteryStatus::Error => 4,
        BatteryStatus::Unknown => 5,
    }
}

fn name_bytes(name: &str, buf: &mut [u8; 256]) {
    let bytes = name.as_bytes();
    let len = bytes.len().min(buf.len() - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
}

fn with_runtime<T, F>(f: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(f).map_err(|e| e.to_string())
}

/// 枚举所有 Logitech HID++ 设备及其电量。
///
/// 成功时 `out` 被填充为设备列表（`devices` 指向堆分配数组，
/// 使用后必须调用 `lb_free_devices` 释放）。
///
/// 返回值：`0` = 成功；负数 = 错误（可用 `lb_last_error` 获取详情）。
#[unsafe(no_mangle)]
pub extern "C" fn lb_enumerate_devices(out: *mut LbDeviceList) -> i32 {
    if out.is_null() {
        set_last_error("null pointer: out");
        return -1;
    }

    let result = with_runtime(async { Ok(enumerate().await?) });

    match result {
        Ok(inventories) => {
            let mut items: Vec<LbDeviceInfo> = Vec::new();
            for inv in &inventories {
                for p in &inv.paired {
                    let mut name = [0u8; 256];
                    let display = p
                        .codename
                        .clone()
                        .unwrap_or_else(|| format!("{} (slot {})", inv.receiver.name, p.slot));
                    name_bytes(&display, &mut name);

                    let (has_battery, percentage, level, status) = match &p.battery {
                        Some(b) => (1, b.percentage, level_to_u8(b.level), status_to_u8(b.status)),
                        None => (0, 0, level_to_u8(BatteryLevel::Unknown), status_to_u8(BatteryStatus::Unknown)),
                    };

                    items.push(LbDeviceInfo {
                        vendor_id: inv.receiver.vendor_id,
                        product_id: inv.receiver.product_id,
                        name,
                        slot: p.slot,
                        online: u8::from(p.online),
                        has_battery,
                        percentage,
                        level,
                        status,
                    });
                }
            }

            let boxed = items.into_boxed_slice();
            let count = boxed.len() as i32;
            let ptr = Box::into_raw(boxed) as *mut LbDeviceInfo;
            unsafe {
                (*out).devices = ptr;
                (*out).count = count;
            }
            0
        }
        Err(e) => {
            set_last_error(&e);
            -2
        }
    }
}

/// 重新读取指定设备的电量。
///
/// `index` 为 `lb_enumerate_devices` 返回列表中的下标。
/// 每次调用都会重新探测（可能耗时数百毫秒）。
///
/// 返回值：`0` = 成功；负数 = 错误。
#[unsafe(no_mangle)]
pub extern "C" fn lb_read_battery(index: i32, out: *mut LbBatteryInfo) -> i32 {
    if index < 0 || out.is_null() {
        set_last_error("invalid index or null pointer: out");
        return -1;
    }

    let result = with_runtime(async { Ok(enumerate().await?) });

    match result {
        Ok(inventories) => {
            let mut flat: Vec<&openlogi_core::device::PairedDevice> = Vec::new();
            for inv in &inventories {
                flat.extend(inv.paired.iter());
            }

            match flat.get(index as usize) {
                Some(p) => match &p.battery {
                    Some(b) => {
                        unsafe {
                            (*out).percentage = b.percentage;
                            (*out).level = level_to_u8(b.level);
                            (*out).status = status_to_u8(b.status);
                        }
                        0
                    }
                    None => {
                        set_last_error("device has no battery data");
                        -3
                    }
                },
                None => {
                    set_last_error("index out of range");
                    -4
                }
            }
        }
        Err(e) => {
            set_last_error(&e);
            -2
        }
    }
}

/// 释放 `lb_enumerate_devices` 返回的设备列表。
///
/// 调用后不得再访问 `list.devices`。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lb_free_devices(list: LbDeviceList) {
    if !list.devices.is_null() && list.count > 0 {
        let _ = unsafe {
            Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                list.devices,
                list.count as usize,
            ))
        };
    }
}

/// 获取最后一次调用的错误信息（NUL 结尾 UTF-8，线程局部）。
///
/// 返回的指针在下次调用任何 `lb_*` 函数前有效。
#[unsafe(no_mangle)]
pub extern "C" fn lb_last_error() -> *const u8 {
    LAST_ERROR.with(|e| e.borrow().as_ptr())
}
