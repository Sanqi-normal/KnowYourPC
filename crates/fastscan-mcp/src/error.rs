use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("此功能仅支持 Windows 平台")]
    Unsupported,
    #[error("需要管理员权限才能读取卷 MFT")]
    NeedAdmin,
    #[error("Windows API 错误: {0}")]
    Win(String),
    #[error("NTFS 解析错误: {0}")]
    Ntfs(String),
    #[error("卷 {0} 不是 NTFS 文件系统")]
    NotNtfs(String),
    #[error("内部错误: {0}")]
    Internal(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
