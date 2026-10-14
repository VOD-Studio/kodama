//! 登录凭据：存在用户配置目录里，只有本人可读，写入是原子替换。
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Creds {
    /// 宠物服务的地址，例如 https://pet.example.com
    pub service: String,
    pub access_token: String,
    /// 访问令牌的过期时间（Unix 秒）。
    pub expires_at: u64,
    pub refresh_token: String,
    pub session_id: String,
}

/// 凭据文件：`$PET_CONFIG_DIR/credentials.json`，缺省在 `$XDG_CONFIG_HOME/kodama` 或 `~/.config/kodama`。
pub fn path() -> Option<PathBuf> {
    let dir = match std::env::var_os("PET_CONFIG_DIR") {
        Some(d) => PathBuf::from(d),
        None => match std::env::var_os("XDG_CONFIG_HOME") {
            Some(x) if !x.is_empty() => PathBuf::from(x).join("kodama"),
            _ => PathBuf::from(std::env::var_os("HOME")?)
                .join(".config")
                .join("kodama"),
        },
    };
    Some(dir.join("credentials.json"))
}

pub fn load() -> Option<Creds> {
    let raw = std::fs::read_to_string(path()?).ok()?;
    serde_json::from_str(&raw).ok()
}

/// 先写同目录的临时文件再改名：并发的另一个进程读到的要么是旧凭据、要么是新凭据，不会是半份。
pub fn save(c: &Creds) -> std::io::Result<()> {
    let path = path().ok_or_else(|| std::io::Error::other("no home directory for credentials"))?;
    let dir = path.parent().expect("credentials live in a directory");
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(".credentials.{}.tmp", std::process::id()));
    {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            opts.mode(0o600);
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        let mut f = opts.open(&tmp)?;
        f.write_all(
            serde_json::to_string_pretty(c)
                .expect("credentials serialize")
                .as_bytes(),
        )?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, &path)
}

pub fn remove() -> std::io::Result<()> {
    match path() {
        Some(p) => match std::fs::remove_file(p) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
        None => Ok(()),
    }
}
