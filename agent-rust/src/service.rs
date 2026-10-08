use std::{env, ffi::OsString, path::PathBuf, process::Command, time::Duration};

use chrono::Utc;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[cfg(windows)]
use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl,
        ServiceExitCode, ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

use crate::acl;

pub const SERVICE_NAME: &str = "FlamingoAgent";

#[cfg(windows)]
define_windows_service!(ffi_service_main, flamingo_service_main);

#[cfg(windows)]
fn flamingo_service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        eprintln!("Service error: {:?}", e);
    }
}

#[cfg(windows)]
fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

    let status_handle = service_control_handler::register(
        SERVICE_NAME,
        move |control_event| match control_event {
            ServiceControl::Stop => {
                let _ = shutdown_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        },
    )?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: Some(std::process::id()),
    })?;

    let rt = tokio::runtime::Runtime::new()?;
    
    rt.block_on(async {
        tokio::select! {
            _ = run_metrics_loop() => {},
            _ = tokio::task::spawn_blocking(move || shutdown_rx.recv()) => {},
        }
    });

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: Some(std::process::id()),
    })?;

    Ok(())
}

pub async fn run_metrics_loop() {
    let mut sys = System::new();
    let pid = Pid::from_u32(std::process::id());

    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(windows)]
    let child_bin = exe_dir.join("logger_child.exe");
    #[cfg(not(windows))]
    let child_bin = exe_dir.join("logger_child");

    let log_file = exe_dir.join("metrics.log");

    if !log_file.exists() {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file);
    }

    if let Err(e) = acl::restrict_to_admin_and_system(&child_bin) {
        eprintln!("Failed to apply ACL permissions on child binary: {}", e);
    }

    if let Err(e) = acl::restrict_to_admin_and_system(&log_file) {
        eprintln!("Failed to apply ACL permissions on log file: {}", e);
    }

    loop {
        sys.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        let rss = sys.process(pid).map(|p| p.memory()).unwrap_or(0);
        let time = Utc::now();
        let arg_str = format!("UTC={} RSS_BYTES={}", time.to_rfc3339(), rss);

        match Command::new(&child_bin)
            .arg(&arg_str)
            .arg(&log_file)
            .status()
        {
            Ok(status) => {
                if !status.success() {
                    eprintln!("Child process exited with non-zero status: {}", status);
                }
            }
            Err(e) => {
                eprintln!("Failed to spawn child process {:?}: {}", child_bin, e);
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

#[cfg(windows)]
pub fn install_windows_service() -> Result<(), Box<dyn std::error::Error>> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CREATE_SERVICE | ServiceManagerAccess::CONNECT,
    )?;

    let exe_path = env::current_exe()?;
    let my_service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from("Flamingo Background Agent"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe_path,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    manager.create_service(&my_service_info, ServiceAccess::QUERY_STATUS)?;
    Ok(())
}

pub fn uninstall_windows_service() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use std::{thread, time::Duration};
        use windows_service::service::{ServiceAccess, ServiceState};
        use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

        let manager = ServiceManager::local_computer(
            None::<&str>,
            ServiceManagerAccess::CONNECT,
        )?;

        let service_access = ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS;

        let service = match manager.open_service(SERVICE_NAME, service_access) {
            Ok(s) => s,
            Err(_) => {
                println!("Service '{}' not found or already uninstalled.", SERVICE_NAME);
                return Ok(());
            }
        };

        if let Ok(status) = service.query_status() {
            if status.current_state != ServiceState::Stopped {
                println!("Stopping '{}' service...", SERVICE_NAME);
                let _ = service.stop();

                for _ in 0..10 {
                    thread::sleep(Duration::from_millis(500));
                    if let Ok(st) = service.query_status() {
                        if st.current_state == ServiceState::Stopped {
                            break;
                        }
                    }
                }
            }
        }

        match service.delete() {
            Ok(_) => println!("Successfully uninstalled '{}' service.", SERVICE_NAME),
            Err(e) => eprintln!("Failed to delete service: {}", e),
        }
    }

    #[cfg(not(windows))]
    {
        eprintln!("Uninstall is only supported on Windows.");
    }

    Ok(())
}

pub fn start_daemon_service() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        if let Err(_e) = service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
            println!("Not running under SCM. Starting background daemon mode...");
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(run_metrics_loop());
        }
    }
    #[cfg(not(windows))]
    {
        println!("Starting background daemon mode...");
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(run_metrics_loop());
    }
    Ok(())
}