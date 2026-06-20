use anyhow::{Result, anyhow};

#[cfg(windows)]
mod platform {
    use super::{Result, anyhow};
    use std::iter;
    use winapi::shared::ntdef::HANDLE;
    use winapi::shared::winerror::ERROR_ALREADY_EXISTS;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::synchapi::CreateMutexW;

    pub struct SingleInstanceGuard {
        handle: HANDLE,
    }

    impl Drop for SingleInstanceGuard {
        fn drop(&mut self) {
            unsafe {
                if !self.handle.is_null() {
                    let _ = CloseHandle(self.handle);
                }
            }
        }
    }

    pub fn acquire() -> Result<SingleInstanceGuard> {
        let name = wide("Local\\ThreeWinDragSingleton");
        unsafe {
            let handle = CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr());
            if handle.is_null() {
                return Err(anyhow!(
                    "3-win-drag could not start (system error). Try restarting your computer if it keeps happening."
                ));
            }

            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(handle);
                return Err(anyhow!(
                    "3-win-drag is already running. Check your system tray for the icon."
                ));
            }

            Ok(SingleInstanceGuard { handle })
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(iter::once(0)).collect()
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{Result, anyhow};
    use std::fs;
    use std::io::{Read, Write};
    use std::path::Path;

    const PID_FILE: &str = "/tmp/3-win-drag.pid";

    pub struct SingleInstanceGuard;

    impl Drop for SingleInstanceGuard {
        fn drop(&mut self) {
            let _ = fs::remove_file(PID_FILE);
        }
    }

    pub fn acquire() -> Result<SingleInstanceGuard> {
        let pid_path = Path::new(PID_FILE);

        // Check if PID file exists and the process is alive
        if pid_path.exists() {
            let mut contents = String::new();
            if let Ok(mut file) = fs::File::open(pid_path) {
                if file.read_to_string(&mut contents).is_ok() {
                    if let Ok(pid) = contents.trim().parse::<i32>() {
                        // Check if process with this PID is running
                        let proc_path = format!("/proc/{pid}");
                        if Path::new(&proc_path).exists() {
                            return Err(anyhow!(
                                "3-win-drag is already running (PID {pid}). Close the existing instance first."
                            ));
                        }
                    }
                }
            }
            // Stale PID file, remove it
            let _ = fs::remove_file(pid_path);
        }

        // Write our PID
        let pid = std::process::id();
        fs::write(pid_path, pid.to_string())
            .map_err(|e| anyhow!("failed to write PID file {PID_FILE}: {e}"))?;

        Ok(SingleInstanceGuard)
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod platform {
    use super::Result;

    pub struct SingleInstanceGuard;

    pub fn acquire() -> Result<SingleInstanceGuard> {
        Ok(SingleInstanceGuard)
    }
}

pub use platform::acquire;
