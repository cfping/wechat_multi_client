use serde::Deserialize;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{env, fs, process, thread};
use tray_item::TrayItem;

// 配置结构体
#[derive(Deserialize, Clone)]
struct Config {
    instance_count: u32,
    wechat_path: String,
    data_dir_prefix: Option<String>,
}

// 从配置文件加载配置，如果没有则使用默认配置
fn load_config() -> Config {
    let config_path = "config.toml";

    let default_config = Config {
        instance_count: 2,
        wechat_path: "C:/Program Files/Tencent/WeChat/WeChat.exe".to_string(),
        data_dir_prefix: None,
    };

    match fs::read_to_string(config_path) {
        Ok(content) => toml::from_str(&content).unwrap_or_else(|_| {
            eprintln!("Error parsing config file, using default config.");
            default_config
        }),
        Err(_) => {
            eprintln!("Config file not found, using default config.");
            default_config
        }
    }
}

fn start_wechat_instance(instance_id: u32, config: &Config) -> Option<Child> {
    let data_dir = match &config.data_dir_prefix {
        Some(prefix) => format!("{}{}", prefix, instance_id),
        None => {
            let user_profile =
                env::var("USERPROFILE").unwrap_or_else(|_| "C:/Users/Default".to_string());
            format!(
                "{}/AppData/Roaming/WeChatInstance{}",
                user_profile, instance_id
            )
        }
    };

    if let Err(e) = fs::create_dir_all(&data_dir) {
        eprintln!("Failed to create directory {}: {}", data_dir, e);
        return None;
    }

    // 启动微信实例
    let child = Command::new(&config.wechat_path)
        // .arg(&data_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start WeChat instance");

    println!(
        "Started WeChat instance {} with PID: {}",
        instance_id,
        child.id()
    );
    Some(child)
}

fn main() {
    let config = load_config();
    let instances = Arc::new(Mutex::new(Vec::new())); // 用于记录启动的实例进程
    let mut tray = TrayItem::new(
        "WeChat Manager",
        tray_item::IconSource::Resource("aa-exe-icon"),
    )
    .unwrap();

    let instances_clone = Arc::clone(&instances);
    let config_clone = config.clone();
    for i in 1..=config.instance_count {
        let config = config.clone();
        if let Some(child) = start_wechat_instance(i, &config) {
            instances_clone.lock().unwrap().push(child);
        }
    }

    // 添加新开 WeChat 实例的菜单项
    tray.add_menu_item("Open New WeChat Instance", move || {
        let instance_id = {
            let mut instances = instances_clone.lock().unwrap();
            instances.len() as u32 + 1
        };
        if let Some(child) = start_wechat_instance(instance_id, &config_clone) {
            instances_clone.lock().unwrap().push(child);
        }
    })
    .unwrap();

    // 添加关闭所有 WeChat 实例的菜单项
    let instances_clone = Arc::clone(&instances);
    tray.add_menu_item("Close All WeChat Instances", move || {
        let mut instances = instances_clone.lock().unwrap();
        while let Some(mut instance) = instances.pop() {
            if let Err(e) = instance.kill() {
                eprintln!("Failed to close WeChat instance: {}", e);
            }
        }
    })
    .unwrap();

    // 添加退出程序的菜单项
    let instances_clone = Arc::clone(&instances);
    tray.add_menu_item("Exit", move || {
        let mut instances = instances_clone.lock().unwrap();
        for mut instance in instances.iter_mut() {
            let _ = instance.kill();
        }
        process::exit(0);
    })
    .unwrap();

    // 主线程延时循环，保持托盘图标常驻
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
