//! HTTP 路由，按资源分文件；路由表在 lib.rs 的 `router`。
//! 新的服务能力（活动收件、好友、繁育、AI 对话）各自成为这里的一个文件。
pub(crate) mod activity;
pub(crate) mod auth;
pub(crate) mod births;
pub(crate) mod device;
pub(crate) mod meta;
pub(crate) mod pets;
pub(crate) mod sessions;
