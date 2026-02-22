#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    #[cfg(feature = "tauri-runtime")]
    anyfast_lib::run()
}
