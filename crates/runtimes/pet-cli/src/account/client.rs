//! 宠物服务的 HTTP 客户端：带访问令牌请求，过期或被拒时用刷新令牌换新再重试一次。
use super::creds::{self, Creds};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 失败的统一形状，与服务的错误体一致：`{ok:false, code, message}`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub code: String,
    pub message: String,
}

impl Failure {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
    pub fn to_json(&self) -> Value {
        json!({"ok": false, "code": self.code, "message": self.message})
    }
    fn logged_out(message: &str) -> Self {
        Self::new("not_logged_in", format!("{message}; run `pet-cli login`"))
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn agent() -> ureq::Agent {
    // ring 作为 rustls 的加密实现；重复安装时返回 Err，忽略即可
    let _ = rustls::crypto::ring::default_provider().install_default();
    let tls = ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::Rustls)
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        // 4xx、5xx 也是带错误体的正常响应，交给调用方按 code 分支
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(format!("pet-cli/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// 发送一个请求，返回（状态码，响应体）。网络错误转成 `network_error`。
pub fn send(
    agent: &ureq::Agent,
    method: &str,
    url: &str,
    bearer: Option<&str>,
    body: Option<&Value>,
) -> Result<(u16, Value), Failure> {
    let network = |e: ureq::Error| Failure::new("network_error", format!("{url}: {e}"));
    let mut resp = match (method, body) {
        ("GET", _) => {
            let mut r = agent.get(url);
            if let Some(t) = bearer {
                r = r.header("authorization", format!("Bearer {t}"));
            }
            r.call().map_err(network)?
        }
        (m, body) => {
            let mut r = match m {
                "DELETE" => agent.delete(url).force_send_body(),
                _ => agent.post(url),
            };
            if let Some(t) = bearer {
                r = r.header("authorization", format!("Bearer {t}"));
            }
            r.content_type("application/json")
                .send(body.map(Value::to_string).unwrap_or_else(|| "{}".into()))
                .map_err(network)?
        }
    };
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().map_err(network)?;
    // 不是 JSON（地址指错了、代理返回的网页）：只报告状态码，不把整页内容塞进错误
    let v = serde_json::from_str(&text).unwrap_or_else(|_| {
        json!({"ok": false, "code": "bad_response",
               "message": format!("{url} answered HTTP {status} without a JSON body; is this a pet service?")})
    });
    Ok((status, v))
}

/// 服务返回的错误体转成 `Failure`。
pub fn failure_of(v: &Value) -> Failure {
    Failure::new(
        v["code"].as_str().unwrap_or("bad_response"),
        v["message"]
            .as_str()
            .unwrap_or("unexpected response from the pet service"),
    )
}

pub struct Client {
    agent: ureq::Agent,
    creds: Creds,
}

impl Client {
    /// 读取已保存的凭据；没有登录时返回 `not_logged_in`。
    pub fn from_saved() -> Result<Self, Failure> {
        let creds = creds::load().ok_or_else(|| Failure::logged_out("not logged in"))?;
        Ok(Self {
            agent: agent(),
            creds,
        })
    }

    /// 带访问令牌的请求，成功时返回 `value`。令牌快过期或被拒时先刷新再重试一次。
    pub fn call(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, Failure> {
        if self.creds.expires_at <= now() + 30 {
            self.refresh()?;
        }
        let url = format!("{}{path}", self.creds.service);
        let (mut status, mut v) = send(
            &self.agent,
            method,
            &url,
            Some(&self.creds.access_token),
            body,
        )?;
        if status == 401 {
            self.refresh()?;
            (status, v) = send(
                &self.agent,
                method,
                &url,
                Some(&self.creds.access_token),
                body,
            )?;
        }
        match (status, v["ok"].as_bool()) {
            (200..=299, Some(true)) => Ok(v["value"].clone()),
            _ => Err(failure_of(&v)),
        }
    }

    /// 用刷新令牌换新令牌并保存。另一个进程刚刷新过（`refresh_raced`）时，改用它保存的新凭据。
    fn refresh(&mut self) -> Result<(), Failure> {
        for attempt in 0..3 {
            let url = format!("{}/v1/auth/refresh", self.creds.service);
            let (status, v) = send(
                &self.agent,
                "POST",
                &url,
                None,
                Some(&json!({"refresh_token": self.creds.refresh_token})),
            )?;
            if status == 200 {
                let t = &v["value"];
                self.creds.access_token = t["access_token"].as_str().unwrap_or_default().into();
                self.creds.expires_at = t["expires_at"].as_u64().unwrap_or(0);
                self.creds.refresh_token = t["refresh_token"].as_str().unwrap_or_default().into();
                creds::save(&self.creds)
                    .map_err(|e| Failure::new("credentials_unwritable", e.to_string()))?;
                return Ok(());
            }
            match v["code"].as_str() {
                Some("refresh_raced") => {
                    // 新凭据由先到的进程写入；稍等后读取
                    std::thread::sleep(Duration::from_millis(200 * (attempt + 1)));
                    if let Some(saved) = creds::load() {
                        if saved.refresh_token != self.creds.refresh_token {
                            self.creds = saved;
                            return Ok(());
                        }
                    }
                }
                Some("refresh_reused") => {
                    let _ = creds::remove();
                    return Err(Failure::logged_out(
                        "the session was revoked because an old token was reused",
                    ));
                }
                _ if status == 401 => {
                    let _ = creds::remove();
                    return Err(Failure::logged_out("the session expired or was signed out"));
                }
                _ => return Err(failure_of(&v)),
            }
        }
        Err(Failure::new(
            "refresh_raced",
            "another pet-cli process is refreshing; try again",
        ))
    }

    /// 登出：撤销会话并删除本地凭据。
    pub fn logout(self) -> Result<bool, Failure> {
        let url = format!("{}/v1/auth/revoke", self.creds.service);
        let result = send(
            &self.agent,
            "POST",
            &url,
            None,
            Some(&json!({"refresh_token": self.creds.refresh_token})),
        );
        creds::remove().map_err(|e| Failure::new("credentials_unwritable", e.to_string()))?;
        let (_, v) = result?;
        Ok(v["value"]["revoked"].as_bool().unwrap_or(false))
    }
}
