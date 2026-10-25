use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use anyhow::{Context, Result, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use rhai::{Dynamic, Engine, EvalAltResult, Scope};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

pub type Headers = Vec<(String, String)>;
#[derive(Clone, Deserialize)]
pub struct Field {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub default: String,
    #[serde(default)]
    pub options: Vec<String>,
}
#[derive(Clone, Deserialize)]
pub struct Manifest {
    pub api_version: u32,
    pub id: String,
    pub name: String,
    pub description: String,
    pub secret_encoding: String,
    pub fields: Vec<Field>,
}
#[derive(Clone)]
pub struct Plugin {
    pub manifest: Manifest,
    script: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub directory: Option<PathBuf>,
    #[serde(default)]
    pub secret_reference: String,
    pub enabled: bool,
    pub selected: String,
    pub values: BTreeMap<String, BTreeMap<String, String>>,
}
pub struct Execution {
    pub plugin: Plugin,
    pub config: BTreeMap<String, String>,
    pub secret: Zeroizing<String>,
}

fn read_bounded(path: &Path) -> Result<String> {
    let mut text = String::new();
    std::fs::File::open(path)?
        .take(65_537)
        .read_to_string(&mut text)?;
    ensure!(text.len() <= 65_536, "插件文件不能超过 64 KiB");
    Ok(text)
}
pub fn builtin_catalog() -> (Vec<Plugin>, Vec<String>) {
    let mut plugins = Vec::new();
    let mut errors = Vec::new();
    for (manifest, script) in [
        (
            include_str!("../plugins/md5-appid-ts/plugin.json"),
            include_str!("../plugins/md5-appid-ts/main.rhai"),
        ),
        (
            include_str!("../plugins/hmac-sha256/plugin.json"),
            include_str!("../plugins/hmac-sha256/main.rhai"),
        ),
        (
            include_str!("../plugins/aes256-gcm/plugin.json"),
            include_str!("../plugins/aes256-gcm/main.rhai"),
        ),
    ] {
        match serde_json::from_str(manifest) {
            Ok(manifest) => plugins.push(Plugin {
                manifest,
                script: script.into(),
            }),
            Err(e) => errors.push(e.to_string()),
        }
    }
    (plugins, errors)
}
pub fn catalog_from(root: &Path) -> (Vec<Plugin>, Vec<String>) {
    let mut plugins = Vec::new();
    let mut errors = Vec::new();
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) => return (plugins, vec![format!("插件目录不可用：{e}")]),
    };
    let mut ids = BTreeSet::new();
    for entry in entries {
        let result = (|| -> Result<Option<Plugin>> {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                return Ok(None);
            }
            let path = entry.path();
            let root = root.canonicalize()?;
            for name in ["plugin.json", "main.rhai"] {
                ensure!(
                    path.join(name).canonicalize()?.starts_with(&root),
                    "插件文件必须位于插件目录内"
                );
            }
            let manifest: Manifest =
                serde_json::from_str(&read_bounded(&path.join("plugin.json"))?)?;
            ensure!(manifest.api_version == 1, "不支持的插件 API 版本");
            ensure!(
                !manifest.id.is_empty() && ids.insert(manifest.id.clone()),
                "插件 ID 为空或重复"
            );
            ensure!(
                matches!(manifest.secret_encoding.as_str(), "utf8" | "hex" | "base64"),
                "未知密钥编码"
            );
            let mut fields = BTreeSet::new();
            ensure!(
                manifest.fields.len() <= 16
                    && manifest.fields.iter().all(|f| !f.key.is_empty()
                        && fields.insert(&f.key)
                        && (f.options.is_empty() || f.options.contains(&f.default))),
                "插件字段配置无效"
            );
            Ok(Some(Plugin {
                manifest,
                script: read_bounded(&path.join("main.rhai"))?,
            }))
        })();
        match result {
            Ok(Some(plugin)) => plugins.push(plugin),
            Ok(None) => {}
            Err(e) => errors.push(e.to_string()),
        }
    }
    plugins.sort_by_key(|p| (p.manifest.id != "md5-appid-ts", p.manifest.name.clone()));
    (plugins, errors)
}
impl Plugin {
    pub fn defaults(&self) -> BTreeMap<String, String> {
        self.manifest
            .fields
            .iter()
            .map(|f| (f.key.clone(), f.default.clone()))
            .collect()
    }
}
fn script_error(message: &'static str) -> Box<EvalAltResult> {
    message.into()
}
fn encrypt(key: &[u8], text: &str, aad: &str) -> Result<String, Box<EvalAltResult>> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|_| script_error("AES-256 密钥必须为 32 字节"))?;
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).map_err(|_| script_error("生成随机数失败"))?;
    let encrypted = cipher
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: text.as_bytes(),
                aad: aad.as_bytes(),
            },
        )
        .map_err(|_| script_error("AES 加密失败"))?;
    Ok(STANDARD.encode([nonce.as_slice(), encrypted.as_slice()].concat()))
}
impl Execution {
    pub fn run(
        self,
        method: &str,
        endpoint: &str,
        body: &str,
        headers: Headers,
        cancel: Arc<AtomicBool>,
    ) -> Result<Headers> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("系统时间不可用")?;
        self.run_at(method, endpoint, body, headers, cancel, now.as_millis())
    }
    fn run_at(
        self,
        method: &str,
        endpoint: &str,
        body: &str,
        headers: Headers,
        cancel: Arc<AtomicBool>,
        millis: u128,
    ) -> Result<Headers> {
        ensure!(!cancel.load(Ordering::Relaxed), "已取消，未发送请求");
        ensure!(
            !self.secret.is_empty(),
            "请填写 secret；密钥仅保留在当前会话"
        );
        ensure!(
            self.secret.len() <= 4096 && body.len() <= 1_048_576,
            "插件输入超过限制（secret 4 KiB / 请求体 1 MiB）"
        );
        ensure!(
            self.config.len() <= 16 && self.config.values().all(|v| v.len() <= 8192),
            "插件参数超过限制"
        );
        let key = Zeroizing::new(match self.plugin.manifest.secret_encoding.as_str() {
            "hex" => hex::decode(self.secret.as_str()).context("secret 必须为十六进制")?,
            "base64" => STANDARD
                .decode(self.secret.as_bytes())
                .context("secret 必须为 Base64")?,
            "utf8" => self.secret.as_bytes().to_vec(),
            _ => anyhow::bail!("不支持的密钥编码"),
        });
        ensure!(!key.is_empty(), "secret 不可为空");
        let key = Arc::new(key);
        let mut engine = Engine::new();
        engine
            .set_max_operations(50_000)
            .set_max_call_levels(16)
            .set_max_expr_depths(32, 32)
            .set_max_string_size(1_048_576)
            .set_max_array_size(256)
            .set_max_map_size(128)
            .set_max_variables(128)
            .set_max_functions(32)
            .disable_symbol("eval");
        engine.on_print(|_| {}).on_debug(|_, _, _| {});
        let started = Instant::now();
        let signal = cancel.clone();
        engine.on_progress(move |_| {
            (signal.load(Ordering::Relaxed) || started.elapsed() > Duration::from_millis(250))
                .then(|| Dynamic::from("插件执行超时或已取消"))
        });
        let md5_key = key.clone();
        engine.register_fn("md5_secret_wrap", move |data: &str| {
            let mut hash = Md5::new();
            hash.update(md5_key.as_slice());
            hash.update(data.as_bytes());
            hash.update(md5_key.as_slice());
            hex::encode(hash.finalize())
        });
        let hmac_key = key.clone();
        engine.register_fn(
            "hmac_sha256",
            move |data: &str| -> Result<String, Box<EvalAltResult>> {
                let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(hmac_key.as_slice())
                    .map_err(|_| script_error("HMAC 密钥无效"))?;
                mac.update(data.as_bytes());
                Ok(hex::encode(mac.finalize().into_bytes()))
            },
        );
        engine.register_fn("aes256_gcm", move |text: &str, aad: &str| {
            encrypt(key.as_slice(), text, aad)
        });
        engine.register_fn("sha256", |text: &str| {
            hex::encode(Sha256::digest(text.as_bytes()))
        });
        let config = rhai::serde::to_dynamic(&self.config)?;
        let request = rhai::serde::to_dynamic(serde_json::json!({
            "method": format!("/{method}"), "endpoint": endpoint, "body": body,
            "timestamp_ms": millis.to_string(), "timestamp": (millis / 1000).to_string()
        }))?;
        let metadata: BTreeMap<_, _> = headers.into_iter().collect();
        ensure!(
            metadata.len() <= 64
                && metadata
                    .iter()
                    .map(|(k, v)| k.len() + v.len())
                    .sum::<usize>()
                    <= 32768,
            "Metadata 超过插件限制"
        );
        let mut scope = Scope::new();
        scope
            .push_constant("config", config)
            .push_constant("request", request)
            .push("metadata", rhai::serde::to_dynamic(metadata)?);
        let output = engine
            .eval_with_scope::<Dynamic>(&mut scope, &self.plugin.script)
            .map_err(|e| anyhow::anyhow!("插件执行失败：{e}"))?;
        ensure!(
            !cancel.load(Ordering::Relaxed) && started.elapsed() <= Duration::from_millis(250),
            "插件已取消或超时，未发送请求"
        );
        let values: BTreeMap<String, String> =
            rhai::serde::from_dynamic(&output).context("插件必须返回字符串值的 Metadata 对象")?;
        ensure!(
            values.len() <= 64
                && values.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() <= 32768,
            "插件输出超过限制（64 个头 / 32 KiB）"
        );
        crate::rpc::metadata(&serde_json::to_string(&values)?, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn execution(id: &str) -> Execution {
        let (plugins, errors) = builtin_catalog();
        assert!(errors.is_empty(), "{errors:?}");
        let plugin = plugins.into_iter().find(|p| p.manifest.id == id).unwrap();
        let mut config = plugin.defaults();
        if id == "md5-appid-ts" {
            config.insert("appid".into(), "demo-app".into());
        }
        Execution {
            plugin,
            config,
            secret: Zeroizing::new("test-secret".into()),
        }
    }
    fn run(execution: Execution, headers: Headers) -> Result<BTreeMap<String, String>> {
        Ok(execution
            .run_at(
                "demo.Service/Call",
                "localhost:50051",
                "{}",
                headers,
                Arc::new(AtomicBool::new(false)),
                1_792_886_400_123,
            )?
            .into_iter()
            .collect())
    }
    #[test]
    fn md5_milliseconds_matches_independent_vector_and_preserves_metadata() {
        let output = run(
            execution("md5-appid-ts"),
            vec![
                ("x-client".into(), "rpcrad".into()),
                ("ts".into(), "stale".into()),
            ],
        )
        .unwrap();
        assert_eq!(
            output,
            BTreeMap::from([
                ("appid".into(), "demo-app".into()),
                ("ts".into(), "1792886400123".into()),
                ("sign".into(), "b5fca2d6ddf38893fe448718cf5feb90".into()),
                ("x-client".into(), "rpcrad".into())
            ])
        );
    }
    #[test]
    fn md5_seconds_uppercase_and_custom_header_names() {
        let mut e = execution("md5-appid-ts");
        for (key, value) in [
            ("timestamp_unit", "seconds"),
            ("hex_case", "upper"),
            ("appid_header", "x-app"),
            ("timestamp_header", "x-ts"),
            ("signature_header", "x-sign"),
        ] {
            e.config.insert(key.into(), value.into());
        }
        assert_eq!(
            run(e, vec![]).unwrap(),
            BTreeMap::from([
                ("x-app".into(), "demo-app".into()),
                ("x-ts".into(), "1792886400".into()),
                ("x-sign".into(), "1D6090E51873B39B3F293CE2451395D0".into())
            ])
        );
    }
    #[test]
    fn missing_appid_secret_and_duplicate_headers_are_rejected() {
        let mut no_appid = execution("md5-appid-ts");
        no_appid.config.insert("appid".into(), "".into());
        let mut no_secret = execution("md5-appid-ts");
        no_secret.secret.clear();
        let mut duplicate = execution("md5-appid-ts");
        duplicate
            .config
            .insert("signature_header".into(), "ts".into());
        for e in [no_appid, no_secret, duplicate] {
            assert!(run(e, vec![]).is_err());
        }
    }
    #[test]
    fn runtime_uses_current_milliseconds() {
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let output: BTreeMap<_, _> = execution("md5-appid-ts")
            .run(
                "S/M",
                "localhost",
                "{}",
                vec![],
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap()
            .into_iter()
            .collect();
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let ts: u128 = output["ts"].parse().unwrap();
        assert!((before..=after).contains(&ts));
    }
    #[test]
    fn hmac_matches_rfc4231_vector() {
        let mut e = execution("hmac-sha256");
        e.secret = Zeroizing::new("\x0b".repeat(20));
        e.plugin.script = "#{ sign: hmac_sha256(\"Hi There\") }".into();
        assert_eq!(
            run(e, vec![]).unwrap()["sign"],
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }
    #[test]
    fn aes_roundtrip_removes_plaintext_and_uses_fresh_nonces() {
        let invoke = || {
            let mut e = execution("aes256-gcm");
            e.secret = Zeroizing::new("01".repeat(32));
            e.config.insert("aad".into(), "demo".into());
            run(e, vec![("x-payload".into(), "hello".into())]).unwrap()
        };
        let first = invoke();
        let second = invoke();
        assert!(!first.contains_key("x-payload"));
        assert_ne!(first["x-encrypted"], second["x-encrypted"]);
        let mut bytes = STANDARD.decode(&first["x-encrypted"]).unwrap();
        let cipher = Aes256Gcm::new_from_slice(&[1u8; 32]).unwrap();
        let nonce: [u8; 12] = bytes[..12].try_into().unwrap();
        assert_eq!(
            cipher
                .decrypt(
                    &Nonce::from(nonce),
                    Payload {
                        msg: &bytes[12..],
                        aad: b"demo"
                    }
                )
                .unwrap(),
            b"hello"
        );
        bytes[12] ^= 1;
        assert!(
            cipher
                .decrypt(
                    &Nonce::from(nonce),
                    Payload {
                        msg: &bytes[12..],
                        aad: b"demo"
                    }
                )
                .is_err()
        );
    }
    #[test]
    fn aes_rejects_wrong_key_size() {
        let mut e = execution("aes256-gcm");
        e.secret = Zeroizing::new("01".repeat(16));
        assert!(run(e, vec![("x-payload".into(), "hello".into())]).is_err());
    }
    #[test]
    fn scripts_are_bounded_and_cannot_import_eval_or_access_secret() {
        for source in [
            "loop {}",
            "import \"file\" as file; metadata",
            "eval(\"metadata\")",
            "#{ leaked: secret }",
            "#{ leaked: read_file(\"/etc/passwd\") }",
        ] {
            let mut e = execution("md5-appid-ts");
            e.plugin.script = source.into();
            assert!(run(e, vec![]).is_err(), "{source}");
        }
    }
    #[test]
    fn plugin_output_rejects_injection_nonstring_and_excessive_size() {
        for source in [
            "#{ sign: \"bad\\nheader\" }",
            "#{ sign: 12 }",
            "#{ Authorization: \"x\" }",
            "#{ sign: \"x\".repeat(32769) }",
        ] {
            let mut e = execution("md5-appid-ts");
            e.plugin.script = source.into();
            assert!(run(e, vec![]).is_err(), "{source}");
        }
    }
    #[test]
    fn cancelled_plugin_does_not_produce_headers() {
        assert!(
            execution("md5-appid-ts")
                .run(
                    "S/M",
                    "localhost",
                    "{}",
                    vec![],
                    Arc::new(AtomicBool::new(true))
                )
                .is_err()
        );
    }
}
