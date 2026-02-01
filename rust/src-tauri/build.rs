use std::fs;

fn main() {
    // 从 tauri.conf.json 读取版本号并设置环境变量
    if let Ok(content) = fs::read_to_string("tauri.conf.json") {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
                println!("cargo:rustc-env=APP_VERSION={}", version);
            }
        }
    }

    // 开发模式不要求管理员权限，发布模式要求
    #[allow(unused_mut)]
    let mut windows = tauri_build::WindowsAttributes::new();

    #[cfg(not(debug_assertions))]
    {
        // 发布模式：要求管理员权限
        windows = windows.app_manifest(include_str!("app.manifest"));
    }

    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri-build");
}
