fn main() {
    // 开发模式不要求管理员权限，发布模式要求
    let mut windows = tauri_build::WindowsAttributes::new();

    #[cfg(not(debug_assertions))]
    {
        // 发布模式：要求管理员权限
        windows = windows.app_manifest(include_str!("app.manifest"));
    }

    tauri_build::try_build(
        tauri_build::Attributes::new().windows_attributes(windows)
    ).expect("failed to run tauri-build");
}
