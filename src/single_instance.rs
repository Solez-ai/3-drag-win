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
                return Err(anyhow!("failed to create application instance mutex"));
            }

            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(handle);
                return Err(anyhow!(
                    "3-win-drag is already running. Close the existing tray instance first."
                ));
            }

            Ok(SingleInstanceGuard { handle })
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(iter::once(0)).collect()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::Result;

    pub struct SingleInstanceGuard;

    pub fn acquire() -> Result<SingleInstanceGuard> {
        Ok(SingleInstanceGuard)
    }
}

pub use platform::acquire;
