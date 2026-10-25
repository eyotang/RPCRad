use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Method {
    pub service: String,
    pub name: String,
    pub source: String,
    pub input: String,
    pub client_stream: bool,
    pub server_stream: bool,
    pub sample: String,
}

impl Method {
    pub fn path(&self) -> String {
        format!("{}/{}", self.service, self.name)
    }
    pub fn kind(&self) -> &'static str {
        match (self.client_stream, self.server_stream) {
            (true, true) => "Bidi stream",
            (true, false) => "Client stream",
            (false, true) => "Server stream",
            _ => "Unary",
        }
    }
}

#[derive(Clone, Default)]
pub struct Connection {
    pub endpoint: String,
    pub tls: bool,
    pub timeout: u64,
    pub protos: Vec<PathBuf>,
    pub metadata: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Reply {
    pub body: String,
    pub headers: String,
    pub trailers: String,
    pub status: String,
    pub elapsed_ms: u128,
    pub bytes: usize,
    pub messages: usize,
}

pub enum Event {
    PluginPrepared(crate::plugin::Headers),
    RequestDispatched(crate::plugin::Headers),
    Progress(String),
    Finished(Reply),
    Discovered(Result<Vec<Method>, String>, Vec<PathBuf>),
}

fn executable() -> PathBuf {
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("grpcurl")));
    if let Some(path) = bundled.filter(|p| p.exists()) {
        return path;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tools/bin/grpcurl")
}

fn command(connection: &Connection, network: bool) -> Command {
    let mut cmd = Command::new(executable());
    cmd.args(["-max-time", &connection.timeout.max(1).to_string()]);
    if network && !connection.tls {
        cmd.arg("-plaintext");
    }
    for (key, value) in &connection.metadata {
        cmd.args(["-H", &format!("{key}: {value}")]);
    }
    let protos: Vec<_> = connection
        .protos
        .iter()
        .map(|p| crate::paths::load_proto(p))
        .collect();
    let parents: BTreeSet<_> = protos.iter().filter_map(|p| p.parent()).collect();
    for parent in parents {
        cmd.arg("-import-path").arg(parent);
    }
    for path in &protos {
        cmd.arg("-proto").arg(path);
    }
    cmd
}

fn inspect(
    connection: &Connection,
    verb: &str,
    symbol: Option<&str>,
    template: bool,
) -> Result<String> {
    let network = connection.protos.is_empty();
    let mut cmd = command(connection, network);
    if template {
        cmd.arg("-msg-template");
    }
    if network {
        cmd.arg(&connection.endpoint);
    }
    cmd.arg(verb);
    if let Some(symbol) = symbol {
        cmd.arg(symbol);
    }
    let output = cmd
        .output()
        .context("无法启动 grpcurl，请先运行 tools/setup.sh")?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn discover(connection: &Connection) -> Result<Vec<Method>> {
    let services = inspect(connection, "list", None, false)?;
    let pattern = Regex::new(
        r"rpc\s+(\w+)\s*\(\s*(stream\s+)?([.\w]+)\s*\)\s*returns\s*\(\s*(stream\s+)?([.\w]+)\s*\)",
    )?;
    let mut methods = Vec::new();
    for service in services
        .lines()
        .filter(|s| !s.starts_with("grpc.reflection.") && !s.is_empty())
    {
        let description = inspect(connection, "describe", Some(service), false)?;
        for captures in pattern.captures_iter(&description) {
            let input = captures[3].trim_start_matches('.').to_owned();
            let template = inspect(connection, "describe", Some(&input), true)?;
            let sample = template
                .find("Message template:")
                .map(|i| template[i + 17..].trim().to_string())
                .unwrap_or_else(|| "{}".to_string());
            let source = connection
                .protos
                .iter()
                .find(|p| {
                    std::fs::read_to_string(p).is_ok_and(|s| {
                        s.contains(&format!(
                            "service {}",
                            service.rsplit('.').next().unwrap_or(service)
                        ))
                    })
                })
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "服务反射".into());
            methods.push(Method {
                service: service.into(),
                name: captures[1].into(),
                source,
                input,
                client_stream: captures.get(2).is_some(),
                server_stream: captures.get(4).is_some(),
                sample,
            });
        }
    }
    if methods.is_empty() {
        bail!("没有找到可调用的方法");
    }
    Ok(methods)
}

pub fn metadata(text: &str, token: &str) -> Result<Vec<(String, String)>> {
    let values: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(text).context("Metadata 必须是 JSON 对象")?;
    let mut entries = Vec::new();
    for (key, value) in values {
        let value = value.as_str().context("Metadata 的值必须是字符串")?;
        if key.is_empty()
            || key.starts_with("grpc-")
            || !key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-.".contains(&b))
        {
            bail!("无效的 Metadata 键：{key}");
        }
        if key.ends_with("-bin") {
            bail!("此版本暂不支持二进制 Metadata");
        }
        if value.bytes().any(|b| !(32..=126).contains(&b)) {
            bail!("Metadata 的值必须是可打印的 ASCII 字符");
        }
        entries.push((key, value.into()));
    }
    if !token.trim().is_empty() {
        if !token.bytes().all(|b| (32..=126).contains(&b)) {
            bail!("Bearer token 包含无效字符");
        }
        entries.retain(|(key, _)| key != "authorization");
        entries.push(("authorization".into(), format!("Bearer {}", token.trim())));
    }
    Ok(entries)
}

pub fn invoke(
    connection: Connection,
    method: Method,
    body: String,
    cancel: Arc<AtomicBool>,
    tx: Sender<Event>,
) {
    let start = Instant::now();
    let result = invoke_inner(connection, method, body, cancel, &tx);
    let reply = match result {
        Ok(mut reply) => {
            reply.elapsed_ms = start.elapsed().as_millis();
            reply
        }
        Err(error) => Reply {
            status: "调用失败".into(),
            body: format!("{error:#}"),
            elapsed_ms: start.elapsed().as_millis(),
            ..Reply::default()
        },
    };
    let _ = tx.send(Event::Finished(reply));
}

fn invoke_inner(
    connection: Connection,
    method: Method,
    body: String,
    cancel: Arc<AtomicBool>,
    tx: &Sender<Event>,
) -> Result<Reply> {
    let mut cmd = command(&connection, true);
    cmd.args([
        "-v",
        "-emit-defaults",
        "-d",
        "@",
        &connection.endpoint,
        &method.path(),
    ]);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("无法启动 grpcurl，请先运行 tools/setup.sh")?;
    // Capture the exact business metadata handed to this invocation, after plugins.
    let _ = tx.send(Event::RequestDispatched(connection.metadata.clone()));
    let mut stdin = child.stdin.take().context("无法打开请求输入")?;
    // Write on a separate thread so a large payload cannot block cancellation.
    thread::spawn(move || {
        let _ = stdin.write_all(body.as_bytes());
    });
    let stdout = child.stdout.take().context("无法读取响应")?;
    let stderr = child.stderr.take().context("无法读取错误")?;
    let progress_tx = tx.clone();
    let reader = thread::spawn(move || {
        let mut output = String::new();
        let mut truncated = false;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if output.len() + line.len() > 8 * 1024 * 1024 {
                truncated = true;
                continue;
            }
            output.push_str(&line);
            output.push('\n');
            let _ = progress_tx.send(Event::Progress(line));
        }
        (output, truncated)
    });
    let errors = thread::spawn(move || {
        let mut text = String::new();
        let _ = BufReader::new(stderr)
            .take(1024 * 1024)
            .read_to_string(&mut text);
        text
    });
    // grpcurl's deadline starts after Proto loading; bound the entire child lifetime too.
    let started = Instant::now();
    let deadline = Duration::from_secs(connection.timeout.max(1).saturating_add(1));
    let mut timed_out = false;
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        if started.elapsed() >= deadline {
            timed_out = true;
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        thread::sleep(Duration::from_millis(25));
    };
    let (output, truncated) = reader.join().unwrap_or_default();
    let error = errors.join().unwrap_or_default();
    let mut reply = parse_output(&output);
    reply.status = match status {
        None if timed_out => "DeadlineExceeded".into(),
        None => "已取消".into(),
        Some(status) if status.success() => "0 OK".into(),
        _ => error
            .lines()
            .find(|l| l.trim().starts_with("Code:"))
            .map(|l| l.trim().trim_start_matches("Code:").trim().to_owned())
            .unwrap_or_else(|| "连接失败".into()),
    };
    if timed_out {
        reply
            .body
            .push_str("\n请求准备或调用超过超时限制；请检查 Proto 文件访问权限及服务连接。");
    }
    if truncated {
        reply.status.push_str(" · 显示已截断");
        reply.body.push_str("\n\n响应超过 8 MiB，后续内容未显示。");
    }
    if !error.trim().is_empty() {
        if !reply.body.is_empty() {
            reply.body.push_str("\n\n");
        }
        reply.body.push_str(error.trim());
    }
    Ok(reply)
}

pub fn parse_output(output: &str) -> Reply {
    let mut reply = Reply::default();
    let mut section = "";
    for line in output.lines() {
        match line {
            "Response headers received:" => {
                section = "headers";
                continue;
            }
            "Response contents:" => {
                section = "body";
                reply.messages += 1;
                if !reply.body.is_empty() {
                    reply.body.push('\n');
                }
                continue;
            }
            "Response trailers received:" => {
                section = "trailers";
                continue;
            }
            _ if line.starts_with("Sent ") || line.starts_with("Received ") => {
                section = "";
                continue;
            }
            _ => {}
        }
        let destination = match section {
            "headers" => &mut reply.headers,
            "body" => &mut reply.body,
            "trailers" => &mut reply.trailers,
            _ => continue,
        };
        destination.push_str(line);
        destination.push('\n');
    }
    reply.body = reply.body.trim().into();
    reply.headers = reply.headers.trim().into();
    reply.trailers = reply.trailers.trim().into();
    reply.bytes = reply.body.len();
    reply
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_stream_messages_and_metadata_separately() {
        let reply = parse_output(
            "Response headers received:\nx-server: demo\n\nResponse contents:\n{\"id\":1}\nResponse contents:\n{\"id\":2}\nResponse trailers received:\nx-end: done\nSent 1 request and received 2 responses\n",
        );
        assert_eq!(
            (
                reply.messages,
                reply.headers.as_str(),
                reply.trailers.as_str()
            ),
            (2, "x-server: demo", "x-end: done")
        );
        assert!(reply.body.contains("{\"id\":2}"));
    }
    #[test]
    fn rejects_non_string_metadata() {
        assert!(metadata(r#"{"foo": 3}"#, "").is_err());
    }
    #[test]
    fn rejects_header_injection() {
        assert!(metadata(r#"{"foo": "a\r\nb"}"#, "").is_err());
    }
    #[test]
    fn bearer_replaces_authorization_metadata() {
        assert_eq!(
            metadata(r#"{"authorization":"old"}"#, "new").unwrap(),
            vec![("authorization".into(), "Bearer new".into())]
        );
    }
}
