use logi_battery::enumerate;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== Logi Battery 测试 ===\n");

    // 枚举设备
    println!("正在枚举 Logitech HID++ 设备...");
    match enumerate().await {
        Ok(inventories) => {
            if inventories.is_empty() {
                println!("未找到 Logitech 设备");
                return;
            }
            println!("找到 {} 个设备:\n", inventories.len());
            for (i, inv) in inventories.iter().enumerate() {
                println!("[{}] {}", i + 1, inv.receiver.name);
                println!("    VID: 0x{:04X}, PID: 0x{:04X}", inv.receiver.vendor_id, inv.receiver.product_id);
                for p in &inv.paired {
                    println!("    - 配对设备 slot={} codename={:?} online={}",
                        p.slot, p.codename, p.online);
                    match &p.battery {
                        Some(b) => println!("      电量: {}% {:?} {:?}", b.percentage, b.level, b.status),
                        None => println!("      无电量数据"),
                    }
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("枚举设备失败: {e}");
            return;
        }
    }
}