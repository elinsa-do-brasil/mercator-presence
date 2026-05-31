use std::ffi::OsString;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use windows_service::define_windows_service;
use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
    ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use super::{SERVICE_DESCRIPTION, SERVICE_DISPLAY_NAME, SERVICE_NAME};
use crate::cli::ServiceCommand;

define_windows_service!(ffi_service_main, service_main);

pub fn handle_command(command: ServiceCommand) -> Result<()> {
    match command {
        ServiceCommand::Install => install(),
        ServiceCommand::Uninstall => uninstall(),
        ServiceCommand::Start => start(),
        ServiceCommand::Stop => stop(),
    }
}

pub fn run_dispatcher() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .context("failed to start Windows Service dispatcher")
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(error) = run_service() {
        tracing::error!(%error, "Windows Service failed");
    }
}

fn run_service() -> Result<()> {
    let _log_guard = crate::logging::init()?;
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let stop_tx_for_handler = stop_tx.clone();

    let status_handle =
        service_control_handler::register(SERVICE_NAME, move |control_event| match control_event {
            ServiceControl::Stop | ServiceControl::Interrogate => {
                if matches!(control_event, ServiceControl::Stop) {
                    let _ = stop_tx_for_handler.send(());
                }
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        })
        .context("failed to register service control handler")?;

    status_handle.set_service_status(service_status(ServiceState::Running))?;
    let runtime = tokio::runtime::Runtime::new().context("failed to create Tokio runtime")?;
    let result = runtime.block_on(crate::app::run_loop(async move {
        let _ = tokio::task::spawn_blocking(move || stop_rx.recv()).await;
    }));

    status_handle.set_service_status(service_status(ServiceState::Stopped))?;
    result
}

fn install() -> Result<()> {
    let manager =
        service_manager(ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE)?;
    let service_binary = std::env::current_exe().context("failed to resolve current executable")?;
    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary,
        launch_arguments: vec![OsString::from("run-service")],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let service = manager
        .create_service(&service_info, ServiceAccess::CHANGE_CONFIG)
        .context("failed to create Windows Service")?;
    service
        .set_description(SERVICE_DESCRIPTION)
        .context("failed to set Windows Service description")?;
    println!("{SERVICE_DISPLAY_NAME} installed as {SERVICE_NAME}");
    Ok(())
}

fn uninstall() -> Result<()> {
    let manager = service_manager(ServiceManagerAccess::CONNECT)?;
    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
        )
        .context("failed to open Windows Service")?;
    service
        .delete()
        .context("failed to delete Windows Service")?;
    println!("{SERVICE_DISPLAY_NAME} uninstalled");
    Ok(())
}

fn start() -> Result<()> {
    let manager = service_manager(ServiceManagerAccess::CONNECT)?;
    let service = manager
        .open_service(SERVICE_NAME, ServiceAccess::START)
        .context("failed to open Windows Service")?;
    service
        .start(&[] as &[OsString])
        .context("failed to start Windows Service")?;
    println!("{SERVICE_DISPLAY_NAME} start requested");
    Ok(())
}

fn stop() -> Result<()> {
    let manager = service_manager(ServiceManagerAccess::CONNECT)?;
    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
        )
        .context("failed to open Windows Service")?;
    service.stop().context("failed to stop Windows Service")?;
    println!("{SERVICE_DISPLAY_NAME} stop requested");
    Ok(())
}

fn service_manager(access: ServiceManagerAccess) -> Result<ServiceManager> {
    ServiceManager::local_computer(None::<&str>, access)
        .context("failed to open Windows Service manager")
}

fn service_status(current_state: ServiceState) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    }
}
