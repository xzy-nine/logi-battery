# logi-battery FFI 接口文档

`logi-battery` 提供 C ABI（`cdylib`）导出，可供 C / C++ / Python (ctypes) /
Rust (extern) 等任何能调用 C 函数的语言使用。底层复用 OpenLogi 的完整
HID++ 实现，无需自行处理协议细节。

## 构建

```bash
cargo build --release
```

产物：

| 平台 | 文件 |
|------|------|
| Windows | `target/release/logi_battery.dll`（另有 `.lib` 导入库） |
| Linux | `target/release/liblogi_battery.so` |
| macOS | `target/release/liblogi_battery.dylib` |

> 注意：库内部会创建 tokio 运行时并阻塞等待枚举/读取完成，
> 单个调用耗时约数百毫秒到数秒，请勿在 UI 主线程中高频调用。

## 数据类型

所有结构体为 C 兼容内存布局（`#[repr(C)]`），字段均为固定大小数组或标量。

```c
/* logi_battery.h */
#include <stdint.h>

/* 设备信息（含电量快照） */
typedef struct {
    uint16_t vendor_id;   /* USB Vendor ID（0x046D = Logitech） */
    uint16_t product_id;  /* USB Product ID */
    uint8_t  name[256];   /* 设备名称，NUL 结尾 UTF-8 */
    uint8_t  slot;        /* 接收器配对槽位（1..=6；直连设备为 0） */
    uint8_t  online;      /* 1 = 在线，0 = 离线 */
    uint8_t  has_battery; /* 1 = 有电量数据 */
    uint8_t  percentage;  /* 电量百分比 0-100 */
    uint8_t  level;       /* 电量等级，见 LbBatteryLevel */
    uint8_t  status;      /* 充电状态，见 LbBatteryStatus */
} LbDeviceInfo;

/* 设备列表 */
typedef struct {
    LbDeviceInfo *devices; /* 堆分配数组，用后须调 lb_free_devices 释放 */
    int32_t       count;   /* 设备数量 */
} LbDeviceList;

/* 电池信息（用于 lb_read_battery） */
typedef struct {
    uint8_t percentage; /* 电量百分比 0-100 */
    uint8_t level;      /* 电量等级 */
    uint8_t status;     /* 充电状态 */
} LbBatteryInfo;

/* 电量等级 */
enum LbBatteryLevel {
    LB_LEVEL_CRITICAL = 0,
    LB_LEVEL_LOW      = 1,
    LB_LEVEL_GOOD     = 2,
    LB_LEVEL_FULL     = 3,
    LB_LEVEL_UNKNOWN  = 4,
};

/* 充电状态 */
enum LbBatteryStatus {
    LB_STATUS_DISCHARGING   = 0,
    LB_STATUS_CHARGING      = 1,
    LB_STATUS_CHARGING_SLOW = 2,
    LB_STATUS_FULL          = 3,
    LB_STATUS_ERROR         = 4,
    LB_STATUS_UNKNOWN       = 5,
};
```

## 函数

### `int32_t lb_enumerate_devices(LbDeviceList *out)`

枚举所有 Logitech HID++ 设备及其电量快照。

- **成功**：`out->devices` 指向新分配的数组，`out->count` 为数量。
  使用完毕后必须调用 `lb_free_devices` 释放。
- **失败**：返回负数，用 `lb_last_error` 获取错误详情。
- 返回值：`0` = 成功，`-1` = 空指针，`-2` = 枚举失败。

### `int32_t lb_read_battery(int32_t index, LbBatteryInfo *out)`

重新读取指定设备的电量（每次调用都会重新探测，可能耗时较长）。

- `index`：`lb_enumerate_devices` 返回列表中的下标。
- 返回值：`0` = 成功，`-1` = 参数无效，`-2` = 枚举失败，
  `-3` = 设备无电量数据，`-4` = 下标越界。

### `void lb_free_devices(LbDeviceList list)`

释放 `lb_enumerate_devices` 返回的列表。释放后不得再访问 `list.devices`。

### `const uint8_t *lb_last_error(void)`

返回最后一次调用的错误信息（NUL 结尾 UTF-8，线程局部）。
指针在下次调用任何 `lb_*` 函数前有效。

## 使用示例

### C

```c
#include <stdio.h>
#include "logi_battery.h"

int main(void) {
    LbDeviceList list = {0};
    if (lb_enumerate_devices(&list) != 0) {
        printf("error: %s\n", (char *)lb_last_error());
        return 1;
    }
    for (int i = 0; i < list.count; i++) {
        LbDeviceInfo *d = &list.devices[i];
        printf("%s: %u%% (level=%d status=%d)\n",
               (char *)d->name, d->percentage, d->level, d->status);
    }
    lb_free_devices(list);
    return 0;
}
```

编译（Windows MinGW 示例，DLL 与可执行文件同目录）：

```bash
gcc demo.c -o demo.exe logi_battery.dll
```

### Python (ctypes)

```python
import ctypes
from ctypes import c_int32, c_uint8, c_uint16, POINTER, Structure

class LbDeviceInfo(Structure):
    _fields_ = [
        ("vendor_id", c_uint16), ("product_id", c_uint16),
        ("name", c_uint8 * 256), ("slot", c_uint8),
        ("online", c_uint8), ("has_battery", c_uint8),
        ("percentage", c_uint8), ("level", c_uint8), ("status", c_uint8),
    ]

class LbDeviceList(Structure):
    _fields_ = [("devices", POINTER(LbDeviceInfo)), ("count", c_int32)]

lib = ctypes.CDLL("logi_battery.dll")
lib.lb_enumerate_devices.argtypes = [POINTER(LbDeviceList)]
lib.lb_enumerate_devices.restype = c_int32
lib.lb_last_error.restype = POINTER(ctypes.c_uint8)

lst = LbDeviceList()
if lib.lb_enumerate_devices(ctypes.byref(lst)) != 0:
    raise RuntimeError(ctypes.string_at(lib.lb_last_error()).decode())

for i in range(lst.count):
    d = lst.devices[i]
    print(ctypes.string_at(d.name, 256).split(b"\0")[0].decode(),
          f"{d.percentage}%", d.level, d.status)

lib.lb_free_devices(lst)
```

### Rust

```rust
use logi_battery::ffi::{lb_enumerate_devices, lb_free_devices, LbDeviceList};

unsafe {
    let mut list = LbDeviceList {
        devices: std::ptr::null_mut(),
        count: 0,
    };
    if lb_enumerate_devices(&mut list) == 0 {
        for i in 0..list.count {
            let d = &*list.devices.add(i as usize);
            println!("{}: {}%", std::ffi::CStr::from_ptr(d.name.as_ptr().cast()).to_string_lossy(), d.percentage);
        }
        lb_free_devices(list);
    }
}
```

## 线程安全

- `lb_last_error` 返回的字符串为**线程局部**存储，不同线程互不干扰。
- 同一进程内可安全地从多个线程并发调用枚举/读取函数
  （内部每次调用创建独立的 tokio 运行时）。
- 多次调用 `lb_enumerate_devices` 必须分别 `lb_free_devices`，
  不要重复释放或释放后继续使用指针。
