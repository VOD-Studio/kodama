//! 账号模式（#15）：经设备授权登录宠物服务，读写与网页同一只主宠。
//! 只经 HTTP 访问服务，不链接服务端 crate；凭据见 `creds`，令牌刷新见 `client`。
//!
//! 每个命令在标准输出打印一行 JSON 结果（`{"ok":true,"value":…}` 或 `{"ok":false,"code":…,"message":…}`），
//! 给人看的提示写到标准错误。成功退出码 0，失败 1，用法错误 2。
mod client;
mod creds;
mod watch;

use client::{failure_of, now, send};
pub use client::{Client, Failure};
use serde_json::{json, Value};
use std::time::Duration;
pub use watch::Watch;

/// `pet-cli <cmd> [args]` 中属于账号模式的命令。
pub const COMMANDS: [&str; 4] = ["login", "logout", "status", "act"];

pub fn run(cmd: &str, args: &[String]) -> i32 {
    let result = match cmd {
        "login" => login(args),
        "logout" => logout(),
        "status" => Client::from_saved().and_then(|mut c| c.call("GET", "/v1/pets/me", None)),
        "act" => match parse_action(args) {
            Ok((action, request_id)) => act(&action, &request_id),
            Err(usage) => {
                eprintln!("{usage}");
                return 2;
            }
        },
        _ => unreachable!("dispatched only for COMMANDS"),
    };
    match result {
        Ok(v) => {
            println!("{}", json!({"ok": true, "value": v}));
            0
        }
        Err(f) => {
            println!("{}", f.to_json());
            1
        }
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn hostname() -> String {
    std::process::Command::new("uname")
        .arg("-n")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown host".into())
}

/// 设备授权登录：打印确认地址与用户码，轮询直到用户在网页上确认、拒绝或过期。
fn login(args: &[String]) -> Result<Value, Failure> {
    let service = flag(args, "--service")
        .or_else(|| std::env::var("PET_SERVICE").ok())
        .or_else(|| creds::load().map(|c| c.service))
        .ok_or_else(|| Failure::new("bad_input", "pass --service URL or set PET_SERVICE"))?;
    let service = service.trim_end_matches('/').to_string();
    if !(service.starts_with("https://") || service.starts_with("http://")) {
        return Err(Failure::new(
            "bad_input",
            "the service must be an http(s) URL",
        ));
    }
    let agent = client::agent();
    let mut name = format!("pet-cli {} on {}", env!("CARGO_PKG_VERSION"), hostname());
    name.truncate(64);
    let (status, v) = send(
        &agent,
        "POST",
        &format!("{service}/v1/device/authorize"),
        None,
        Some(&json!({ "client": name })),
    )?;
    if status != 200 {
        return Err(failure_of(&v));
    }
    let g = &v["value"];
    let (uri, code) = (
        g["verification_uri_complete"].as_str().unwrap_or_default(),
        g["user_code"].as_str().unwrap_or_default(),
    );
    eprintln!("To sign in, open this page and confirm the code {code}:\n\n    {uri}\n\nWaiting for confirmation… (Ctrl+C to cancel)");
    // PET_LOGIN_POLL_MS 只供测试缩短等待
    let mut interval = std::env::var("PET_LOGIN_POLL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_secs(g["interval"].as_u64().unwrap_or(5)));
    let deadline = now() + g["expires_in"].as_u64().unwrap_or(600);
    let body = json!({ "device_code": g["device_code"] });
    loop {
        std::thread::sleep(interval);
        if now() > deadline {
            return Err(Failure::new(
                "expired_token",
                "the code expired before it was confirmed; run login again",
            ));
        }
        let (status, v) = send(
            &agent,
            "POST",
            &format!("{service}/v1/device/token"),
            None,
            Some(&body),
        )?;
        if status == 200 {
            let t = &v["value"];
            let c = creds::Creds {
                service: service.clone(),
                access_token: t["access_token"].as_str().unwrap_or_default().into(),
                expires_at: t["expires_at"].as_u64().unwrap_or(0),
                refresh_token: t["refresh_token"].as_str().unwrap_or_default().into(),
                session_id: t["session_id"].as_str().unwrap_or_default().into(),
            };
            creds::save(&c).map_err(|e| Failure::new("credentials_unwritable", e.to_string()))?;
            eprintln!("Signed in.");
            return Ok(json!({ "service": service, "session_id": c.session_id }));
        }
        match v["code"].as_str() {
            Some("authorization_pending") => {}
            Some("slow_down") => interval += Duration::from_secs(5),
            _ => return Err(failure_of(&v)),
        }
    }
}

fn logout() -> Result<Value, Failure> {
    let revoked = Client::from_saved()?.logout()?;
    eprintln!("Signed out.");
    Ok(json!({ "revoked": revoked }))
}

/// `act <feed|rest|play|heal|wait|evolve BRANCH> [--request-id ID]`。
fn parse_action(args: &[String]) -> Result<(Value, String), String> {
    const USAGE: &str =
        "usage: pet-cli act <feed|rest|play|heal|wait|evolve BRANCH> [--request-id ID]";
    let action = match args.first().map(String::as_str) {
        Some(a @ ("feed" | "rest" | "play" | "heal" | "wait")) => json!(a),
        Some("evolve") => match args.get(1) {
            Some(branch) if !branch.starts_with('-') => json!({ "evolve": { "branch": branch } }),
            _ => return Err(USAGE.into()),
        },
        _ => return Err(USAGE.into()),
    };
    // 缺省的请求标识每次不同；需要安全重试时自己传同一个
    let request_id = flag(args, "--request-id").unwrap_or_else(new_request_id);
    Ok((action, request_id))
}

fn act(action: &Value, request_id: &str) -> Result<Value, Failure> {
    act_as(&mut Client::from_saved()?, action, request_id)
}

fn act_as(client: &mut Client, action: &Value, request_id: &str) -> Result<Value, Failure> {
    client.call(
        "POST",
        "/v1/pets/me/actions",
        Some(&json!({ "action": action, "request_id": request_id })),
    )
}

/// 用新的请求标识执行一个动作（`live --account` 的按键）。
pub(crate) fn act_with(client: &mut Client, action: &Value) -> Result<Value, Failure> {
    act_as(client, action, &new_request_id())
}

fn new_request_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("cli-{nanos:x}-{:x}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn actions_parse_into_the_wire_shape() {
        let (a, id) = parse_action(&args(&["feed", "--request-id", "r1"])).unwrap();
        assert_eq!((a, id.as_str()), (json!("feed"), "r1"));
        let (a, id) = parse_action(&args(&["evolve", "curious"])).unwrap();
        assert_eq!(a, json!({"evolve": {"branch": "curious"}}));
        assert!(id.starts_with("cli-"));
        assert!(parse_action(&args(&["evolve"])).is_err());
        assert!(parse_action(&args(&["evolve", "--request-id"])).is_err());
        assert!(parse_action(&args(&["fly"])).is_err());
        assert!(parse_action(&[]).is_err());
    }

    #[test]
    fn default_request_ids_are_valid_for_the_service() {
        let (_, id) = parse_action(&args(&["wait"])).unwrap();
        assert!(id.len() <= 64);
        assert!(id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b)));
    }
}
