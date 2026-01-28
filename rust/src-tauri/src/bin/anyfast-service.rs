//! anyFAST Windows Service
//!
//! This is the service executable that manages hosts file operations
//! with elevated privileges. It communicates with the GUI via Named Pipes.

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::OsString;
    use std::sync::mpsc;
    use std::time::Duration;
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };

    const SERVICE_NAME: &str = "anyfast-service";
    const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

    // Generate the windows service boilerplate
    define_windows_service!(ffi_service_main, service_main);

    fn service_main(_arguments: Vec<OsString>) {
        if let Err(e) = run_service() {
            // Log error - in production, use Windows Event Log
            eprintln!("Service error: {}", e);
        }
    }

    fn run_service() -> Result<(), Box<dyn std::error::Error>> {
        // Create a channel to receive stop signal
        let (shutdown_tx, shutdown_rx) = mpsc::channel();

        // Define the service control handler
        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop => {
                    // Signal the service to stop
                    let _ = shutdown_tx.send(());
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        // Register system service event handler
        let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

        // Report service is starting
        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::StartPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(10),
            process_id: None,
        })?;

        // Create and start the pipe server
        let server = anyfast_lib::service::pipe_server::PipeServer::new();
        let server_clone = std::sync::Arc::new(server);
        let server_for_thread = server_clone.clone();

        // Run pipe server in a separate thread
        let server_thread = std::thread::spawn(move || {
            if let Err(e) = server_for_thread.run() {
                eprintln!("Pipe server error: {}", e);
            }
        });

        // Report running
        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;

        // Wait for stop signal
        let _ = shutdown_rx.recv();

        // Report stopping
        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::StopPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(10),
            process_id: None,
        })?;

        // Stop the pipe server
        server_clone.stop();

        // Wait for server thread (with timeout)
        let _ = server_thread.join();

        // Report stopped
        status_handle.set_service_status(ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;

        Ok(())
    }

    // Check if running as a service or in console mode
    // When run from console, allow manual testing
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--console" {
        // Console mode for debugging
        println!("anyFAST Service - Console Mode");
        println!("Press Ctrl+C to stop");

        let server = anyfast_lib::service::pipe_server::PipeServer::new();

        // Set up Ctrl+C handler
        let server_clone = std::sync::Arc::new(server);
        let server_for_signal = server_clone.clone();

        ctrlc::set_handler(move || {
            println!("\nStopping service...");
            server_for_signal.stop();
        })?;

        server_clone.run().map_err(|e| e.into())
    } else if args.len() > 1 && args[1] == "install" {
        // Install the service
        install_service()
    } else if args.len() > 1 && args[1] == "uninstall" {
        // Uninstall the service
        uninstall_service()
    } else {
        // Run as Windows service
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        Ok(())
    }
}

#[cfg(windows)]
fn install_service() -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::OsString;
    use windows_service::{
        service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;

    let service_binary_path = std::env::current_exe()?;

    let service_info = ServiceInfo {
        name: OsString::from("anyfast-service"),
        display_name: OsString::from("anyFAST Hosts Service"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // Run as LocalSystem
        account_password: None,
    };

    let service = manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;

    // Set description
    service.set_description("Manages hosts file for anyFAST network optimization tool")?;

    println!("Service installed successfully!");
    println!("Start the service with: sc start anyfast-service");

    Ok(())
}

#[cfg(windows)]
fn uninstall_service() -> Result<(), Box<dyn std::error::Error>> {
    use windows_service::{
        service::ServiceAccess,
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service("anyfast-service", ServiceAccess::DELETE)?;

    service.delete()?;

    println!("Service uninstalled successfully!");

    Ok(())
}

#[cfg(not(windows))]
fn main() {
    eprintln!("This service is only supported on Windows");
    std::process::exit(1);
}
