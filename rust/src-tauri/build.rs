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

    // 仅在 tauri-runtime feature 启用且非测试环境下运行 tauri_build
    // - CARGO_FEATURE_TAURI_RUNTIME: 检测 feature 是否启用（--no-default-features 时不设置）
    // - CARGO_CFG_TEST: 检测是否 cargo test 环境
    // 两者任一缺失都跳过 tauri_build，避免：
    //   1. tauri 依赖缺失时 build.rs panic
    //   2. Windows GUI 资源链接导致测试 STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)
    if std::env::var("CARGO_FEATURE_TAURI_RUNTIME").is_ok()
        && std::env::var("CARGO_CFG_TEST").is_err()
    {
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
}
