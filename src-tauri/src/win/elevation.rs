#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    const TOKEN_ELEVATION: i32 = 20;

    unsafe {
        let process_handle = GetCurrentProcess();
        let mut token_handle = std::ptr::null_mut();

        if OpenProcessToken(process_handle, TOKEN_QUERY, &mut token_handle) == 0 {
            return false;
        }

        let mut elevation: u32 = 0;
        let mut returned = 0u32;
        let size = std::mem::size_of::<u32>() as u32;

        let ok = GetTokenInformation(
            token_handle,
            TOKEN_ELEVATION,
            &mut elevation as *mut _ as *mut std::ffi::c_void,
            size,
            &mut returned,
        );

        ok != 0 && elevation != 0
    }
}

#[cfg(windows)]
pub fn restart_elevated() -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;

    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let wide: Vec<u16> = OsStr::new(&exe_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let verb: Vec<u16> = OsStr::new("runas")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            5,
        )
    };

    let code = result as isize;
    if code <= 32 {
        if code == 1223 {
            return Err("用户取消了提权请求".into());
        }
        return Err(format!("提权失败 (错误码: {})", code));
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(not(windows))]
pub fn restart_elevated() -> Result<(), String> {
    Err("不支持的操作系统".into())
}
