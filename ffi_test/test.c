#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* 与 src/ffi.rs 中的 #[repr(C)] 结构一一对应 */
typedef struct {
    uint16_t vendor_id;
    uint16_t product_id;
    uint8_t name[256];
    uint8_t slot;
    uint8_t online;
    uint8_t has_battery;
    uint8_t percentage;
    uint8_t level;
    uint8_t status;
} LbDeviceInfo;

typedef struct {
    LbDeviceInfo *devices;
    int32_t count;
} LbDeviceList;

typedef struct {
    uint8_t percentage;
    uint8_t level;
    uint8_t status;
} LbBatteryInfo;

/* 导出的 C 函数 */
int32_t lb_enumerate_devices(LbDeviceList *out);
void lb_free_devices(LbDeviceList list);
const uint8_t *lb_last_error(void);
int32_t lb_read_battery(int32_t index, LbBatteryInfo *out);

static const char *level_str(uint8_t level) {
    switch (level) {
        case 0: return "Critical";
        case 1: return "Low";
        case 2: return "Good";
        case 3: return "Full";
        default: return "Unknown";
    }
}

static const char *status_str(uint8_t status) {
    switch (status) {
        case 0: return "Discharging";
        case 1: return "Charging";
        case 2: return "ChargingSlow";
        case 3: return "Full";
        case 4: return "Error";
        default: return "Unknown";
    }
}

int main(void) {
    printf("=== logi-battery FFI test ===\n\n");

    /* 1. 枚举设备 */
    LbDeviceList list = {0};
    int32_t rc = lb_enumerate_devices(&list);
    if (rc != 0) {
        printf("lb_enumerate_devices failed (%d): %s\n", rc, (char *)lb_last_error());
        return 1;
    }

    printf("found %d device(s)\n\n", list.count);
    for (int i = 0; i < list.count; i++) {
        LbDeviceInfo *d = &list.devices[i];
        printf("[%d] %s\n", i, (char *)d->name);
        printf("    vid=0x%04X pid=0x%04X slot=%d online=%d\n",
               d->vendor_id, d->product_id, d->slot, d->online);
        if (d->has_battery) {
            printf("    battery: %u%% %s %s\n",
                   d->percentage, level_str(d->level), status_str(d->status));
        } else {
            printf("    battery: none\n");
        }
    }

    /* 2. 重新读取第一个设备的电量 */
    if (list.count > 0) {
        printf("\nre-reading battery of device[0]...\n");
        LbBatteryInfo info;
        rc = lb_read_battery(0, &info);
        if (rc == 0) {
            printf("    battery: %u%% %s %s\n",
                   info.percentage, level_str(info.level), status_str(info.status));
        } else {
            printf("    lb_read_battery failed (%d): %s\n", rc, (char *)lb_last_error());
        }
    }

    /* 3. 越界访问应报错 */
    printf("\ntesting out-of-range index...\n");
    LbBatteryInfo info;
    rc = lb_read_battery(999, &info);
    printf("    rc=%d error=%s\n", rc, (char *)lb_last_error());

    /* 4. 释放 */
    lb_free_devices(list);
    printf("\nlb_free_devices done. OK\n");
    return 0;
}
