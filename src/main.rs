use std::{env, fs, process, sync::{Arc, Mutex}};
use std::process::{Command, Stdio, Child};
use serde::Deserialize;
use tray_item::TrayItem;
use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ControlFlow, EventLoop}, window::{Window, WindowAttributes, WindowId}
};

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

// 启动 WeChat 实例
fn start_wechat_instance(instance_id: u32, config: &Config) -> Option<Child> {
    let data_dir = match &config.data_dir_prefix {
        Some(prefix) => format!("{}{}", prefix, instance_id),
        None => {
            let user_profile = env::var("USERPROFILE").unwrap_or_else(|_| "C:/Users/Default".to_string());
            format!("{}/AppData/Roaming/WeChatInstance{}", user_profile, instance_id)
        }
    };

    if let Err(e) = fs::create_dir_all(&data_dir) {
        eprintln!("Failed to create directory {}: {}", data_dir, e);
        return None;
    }

    let child = Command::new(&config.wechat_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start WeChat instance");

    println!("Started WeChat instance {} with PID: {}", instance_id, child.id());
    Some(child)
}

// 应用程序结构体，管理托盘和窗口事件
struct App {
    window_id: Option<WindowId>,
    instances: Arc<Mutex<Vec<Child>>>,
    window :Option<Window>
}

impl App {
    fn new(instances: Arc<Mutex<Vec<Child>>>) -> Self {
        Self { window_id:None, instances,window:None }
    }
}
impl  ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // let window_attributes = Window::default_attributes().with_title("A fantastic window!");
        
        // self.window = Some(event_loop.create_window(window_attributes).unwrap());
       
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Closing Window={window_id:?}");
                event_loop.set_control_flow(ControlFlow::Wait);
            },
            _=>{
                log::warn!("Closing Window={window_id:?}");
            }
        }
      
    }
}

fn main() {
    let config = load_config();
    let instances = Arc::new(Mutex::new(Vec::new()));
    let mut tray = TrayItem::new("WeChat Manager", tray_item::IconSource::Resource("aa-exe-icon")).unwrap();

    let instances_clone = Arc::clone(&instances);
    let config_clone = config.clone();

    tray.add_menu_item("Open New WeChat Instance", move || {
        let instance_id = {
            let mut instances = instances_clone.lock().unwrap();
            instances.len() as u32 + 1
        };
        if let Some(child) = start_wechat_instance(instance_id, &config_clone) {
            instances_clone.lock().unwrap().push(child);
        }
    }).unwrap();

    let instances_clone = Arc::clone(&instances);
    tray.add_menu_item("Close All WeChat Instances", move || {
        let mut instances = instances_clone.lock().unwrap();
        while let Some(mut instance) = instances.pop() {
            let _ = instance.kill();
        }
    }).unwrap();

    let instances_clone = Arc::clone(&instances);
    tray.add_menu_item("Exit", move || {
        let mut instances = instances_clone.lock().unwrap();
        for mut instance in instances.iter_mut() {
            let _ = instance.kill();
        }
        process::exit(0);
    }).unwrap();

    // 创建事件循环和窗口
    let event_loop = EventLoop::new().unwrap();
    // let window_attributes = WindowAttributes::default().with_title(
    //     "Press 1, 2, 3 to change control flow mode. Press R to toggle redraw requests.",
    // );
    // let window =event_loop.create_window(window_attributes).unwrap();
    // let window_id = window.id();

    // 创建应用实例并启动事件循环
    let mut app = App::new(instances.clone());
    let _ = event_loop.run_app(&mut app);
}
