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
pub fn elevate_self(port: u16) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("无法获取可执行文件路径: {e}"))?;
    let cwd = std::env::current_dir()
        .map_err(|e| format!("无法获取当前工作目录: {e}"))?;

    let params = format!("--port {port}");

    let exe_wide: Vec<u16> = OsStr::new(exe_path.as_os_str())
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let params_wide: Vec<u16> = OsStr::new(&params)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let dir_wide: Vec<u16> = OsStr::new(cwd.as_os_str())
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb_wide: Vec<u16> = OsStr::new("runas")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb_wide.as_ptr(),
            exe_wide.as_ptr(),
            params_wide.as_ptr(),
            dir_wide.as_ptr(),
            0, // SW_HIDE
        )
    };

    let ret = result as isize;
    if ret <= 32 {
        let msg = if ret == 1223 {
            "用户取消了提权请求".to_string()
        } else {
            format!("提权启动失败 (ShellExecute 返回 {ret})")
        };
        return Err(msg);
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    true
}

#[cfg(not(windows))]
pub fn elevate_self(_port: u16) -> Result<(), String> {
    Err("此功能仅支持 Windows 平台".to_string())
}
