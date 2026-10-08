mod acl;
mod service;

use clap::Parser;
use service::*;

#[derive(Parser, Debug)]
#[command(about = "Flamingo Agent Service", long_about = None)]
struct Args {
    /// Install the service (Windows only)
    #[arg(short, long)]
    install: bool,
    /// Uninstall the service (Windows only)
    #[arg(short, long)]
    uninstall: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.uninstall {
        #[cfg(windows)]
        {
            match uninstall_windows_service() {
                Ok(_) => println!("Successfully uninstalled '{}' service.", SERVICE_NAME),
                Err(e) => eprintln!("Failed to uninstall '{}' service: {}", SERVICE_NAME, e),
                
            };
            return Ok(());
        }
        #[cfg(not(windows))]
        {
            eprintln!("--uninstall flag is only supported on Windows.");
            return Ok(());
        }
    }

    if args.install {
        #[cfg(windows)]
        {
            match install_windows_service() {
                Ok(_) => println!("Successfully installed '{}' service.", SERVICE_NAME),
                Err(e) => eprintln!("Failed to install '{}' service: {}", SERVICE_NAME, e),
            }
            return Ok(());
        }
        #[cfg(not(windows))]
        {
            eprintln!("--install flag is only supported on Windows.");
            return Ok(());
        }
    }

    start_daemon_service()?;
    Ok(())
}
