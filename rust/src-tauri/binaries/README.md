# Service Binary Placeholder

The `anyfast-service.exe` binary should be placed here before building the installer.

## Build Instructions

1. Build the service binary:
   ```bash
   cd ../
   cargo build --release --bin anyfast-service
   ```

2. Copy the binary to this directory with platform suffix:
   ```bash
   # For Windows x86_64
   cp target/release/anyfast-service.exe binaries/anyfast-service-x86_64-pc-windows-msvc.exe
   ```

3. Build the Tauri application:
   ```bash
   npm run tauri build
   ```

## CI/CD

In CI/CD, add these steps to your build script:

```yaml
- name: Build Service Binary
  run: cargo build --release --bin anyfast-service

- name: Copy Service Binary
  run: |
    mkdir -p src-tauri/binaries
    cp target/release/anyfast-service.exe src-tauri/binaries/anyfast-service-x86_64-pc-windows-msvc.exe

- name: Build Tauri App
  run: npm run tauri build
```
