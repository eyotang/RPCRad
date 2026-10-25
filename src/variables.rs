//! Workspace variables and request-time interpolation, independent of the UI.
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;

const LIMIT: usize = 1_048_576;

#[derive(Clone, Default, Deserialize)]
pub struct Variable {
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub sensitive: bool,
}

// Keep the name and type of credentials, but never persist their session value.
impl Serialize for Variable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Stored<'a> {
            name: &'a str,
            value: &'a str,
            sensitive: bool,
        }
        Stored {
            name: &self.name,
            value: if self.sensitive { "" } else { &self.value },
            sensitive: self.sensitive,
        }
        .serialize(serializer)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    pub id: u64,
    pub name: String,
    pub variables: Vec<Variable>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Variables {
    pub globals: Vec<Variable>,
    pub environments: Vec<Environment>,
    pub active: Option<u64>,
}

impl Variables {
    pub fn active_name(&self) -> &str {
        self.environments
            .iter()
            .find(|e| Some(e.id) == self.active)
            .map_or("无环境 · 全局变量", |e| e.name.as_str())
    }
    pub fn resolver(&self) -> Result<Resolver<'_>> {
        let mut values = BTreeMap::new();
        let add = |rows: &[Variable]| -> Result<()> {
            let mut names = std::collections::BTreeSet::new();
            for row in rows {
                if row.name.trim().is_empty() && row.value.is_empty() {
                    continue;
                }
                let name = row.name.trim();
                ensure!(
                    valid_name(name),
                    "变量名只能包含字母、数字、下划线、点和短横线，且不能以数字开头"
                );
                ensure!(names.insert(name.to_owned()), "变量名重复：{name}");
            }
            Ok(())
        };
        add(&self.globals)?;
        for v in &self.globals {
            if !v.name.trim().is_empty() {
                values.insert(v.name.trim(), v);
            }
        }
        if let Some(id) = self.active {
            let env = self
                .environments
                .iter()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("当前环境不存在，请重新选择"))?;
            add(&env.variables)?;
            for v in &env.variables {
                if !v.name.trim().is_empty() {
                    values.insert(v.name.trim(), v);
                }
            }
        }
        Ok(Resolver { values })
    }
}

fn valid_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && bytes.all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}

pub fn is_reference(text: &str) -> bool {
    text.strip_prefix("{{")
        .and_then(|s| s.strip_suffix("}}"))
        .is_some_and(|name| valid_name(name.trim()))
}

pub struct Resolver<'a> {
    values: BTreeMap<&'a str, &'a Variable>,
}
impl Resolver<'_> {
    pub fn text(&self, input: &str) -> Result<String> {
        self.expand(input, false, &mut Vec::new(), &mut 4096)
    }
    pub fn json(&self, input: &str) -> Result<String> {
        let output = self.expand(input, true, &mut Vec::new(), &mut 4096)?;
        serde_json::from_str::<serde_json::Value>(&output).map_err(|e| {
            anyhow::anyhow!(
                "变量替换后的 JSON 格式有误（第 {} 行，第 {} 列）",
                e.line(),
                e.column()
            )
        })?;
        Ok(output)
    }
    fn value(&self, name: &str, stack: &mut Vec<String>, budget: &mut usize) -> Result<String> {
        ensure!(valid_name(name), "无效的变量引用，请使用 {{{{变量名}}}}");
        ensure!(stack.len() < 16, "变量引用层级超过 16 层");
        ensure!(!stack.iter().any(|s| s == name), "变量循环引用：{name}");
        let variable = self
            .values
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("未定义变量：{name}"))?;
        ensure!(
            !variable.sensitive || !variable.value.is_empty(),
            "敏感变量尚未填写：{name}"
        );
        stack.push(name.to_string());
        let result = self.expand(&variable.value, false, stack, budget);
        stack.pop();
        result
    }
    fn expand(
        &self,
        input: &str,
        json: bool,
        stack: &mut Vec<String>,
        budget: &mut usize,
    ) -> Result<String> {
        ensure!(input.len() <= LIMIT, "变量或请求内容超过 1 MiB");
        let mut output = String::with_capacity(input.len());
        let mut offset = 0;
        let mut quoted = false;
        let mut escaped = false;
        while offset < input.len() {
            let rest = &input[offset..];
            if let Some(reference) = rest.strip_prefix("{{") {
                let Some(end) = reference.find("}}") else {
                    bail!("变量引用缺少结束符号 }}}}");
                };
                ensure!(!json || !escaped, "变量引用前不能直接使用反斜杠");
                let name = reference[..end].trim();
                ensure!(*budget > 0, "变量引用次数超过限制（4096 次）");
                *budget -= 1;
                let value = self.value(name, stack, budget)?;
                if json && quoted {
                    let encoded = serde_json::to_string(&value)?;
                    output.push_str(&encoded[1..encoded.len() - 1]);
                } else if json {
                    let parsed: serde_json::Value = serde_json::from_str(&value).map_err(|_| {
                        anyhow::anyhow!(
                            "变量 {name} 不是有效的 JSON 值；字符串请写为 \"{{{{{name}}}}}\""
                        )
                    })?;
                    output.push_str(&serde_json::to_string(&parsed)?);
                } else {
                    output.push_str(&value);
                }
                offset += end + 4;
            } else {
                let c = rest
                    .chars()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("无效文本"))?;
                output.push(c);
                offset += c.len_utf8();
                if json {
                    if escaped {
                        escaped = false;
                    } else if quoted && c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        quoted = !quoted;
                    }
                }
            }
            ensure!(output.len() <= LIMIT, "变量展开后的内容超过 1 MiB");
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn variable(name: &str, value: &str) -> Variable {
        Variable {
            name: name.into(),
            value: value.into(),
            sensitive: false,
        }
    }
    fn variables() -> Variables {
        Variables {
            globals: vec![
                variable("appid", "global"),
                variable("host", "localhost"),
                variable("endpoint", "{{host}}:50051"),
            ],
            environments: vec![Environment {
                id: 1,
                name: "测试".into(),
                variables: vec![variable("appid", "test"), variable("host", "test.internal")],
            }],
            active: Some(1),
        }
    }
    #[test]
    fn environment_overrides_globals_and_resolves_nested_references() {
        let mut vars = variables();
        assert_eq!(
            vars.resolver()
                .unwrap()
                .text("{{ endpoint }}/{{appid}}")
                .unwrap(),
            "test.internal:50051/test"
        );
        vars.active = None;
        assert_eq!(
            vars.resolver()
                .unwrap()
                .text("{{endpoint}}/{{appid}}")
                .unwrap(),
            "localhost:50051/global"
        );
    }
    #[test]
    fn json_strings_are_escaped_and_unquoted_values_keep_their_types() {
        let vars = Variables {
            globals: vec![
                variable("name", "引号\"\n反斜杠\\"),
                variable("n", "42"),
                variable("yes", "true"),
                variable("obj", r#"{"a":[1,2]}"#),
            ],
            ..Default::default()
        };
        let result = vars
            .resolver()
            .unwrap()
            .json(r#"{"name":"hello {{name}}", "n":{{n}}, "b":{{yes}}, "o":{{obj}}}"#)
            .unwrap();
        let result: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            result,
            serde_json::json!({"name":"hello 引号\"\n反斜杠\\", "n":42,"b":true,"o":{"a":[1,2]}})
        );
    }
    #[test]
    fn missing_cycles_duplicates_and_invalid_json_fail_without_values_in_errors() {
        let mut vars = variables();
        assert!(
            vars.resolver()
                .unwrap()
                .text("{{missing}}")
                .unwrap_err()
                .to_string()
                .contains("missing")
        );
        vars.globals.push(variable("cycle", "{{cycle}}"));
        assert!(vars.resolver().unwrap().text("{{cycle}}").is_err());
        vars.globals.push(variable("host", "duplicate"));
        assert!(vars.resolver().is_err());
        let vars = Variables {
            globals: vec![variable("bad", "private-invalid-json")],
            ..Default::default()
        };
        let error = vars
            .resolver()
            .unwrap()
            .json("{\"x\":{{bad}}}")
            .unwrap_err()
            .to_string();
        assert!(!error.contains("private-invalid-json"));
    }
    #[test]
    fn secret_values_are_session_only_but_names_and_references_persist() {
        let vars = Variables {
            globals: vec![
                Variable {
                    name: "secret".into(),
                    value: "private-value".into(),
                    sensitive: true,
                },
                variable("signature_key", "{{secret}}"),
            ],
            ..Default::default()
        };
        let stored = serde_json::to_string(&vars).unwrap();
        assert!(!stored.contains("private-value"));
        let restored: Variables = serde_json::from_str(&stored).unwrap();
        assert!(
            restored
                .resolver()
                .unwrap()
                .text("{{signature_key}}")
                .is_err()
        );
        assert_eq!(
            vars.resolver().unwrap().text("{{signature_key}}").unwrap(),
            "private-value"
        );
    }
    #[test]
    fn expansion_is_bounded_and_literal_requests_are_unchanged() {
        let vars = Variables {
            globals: vec![variable("a", &"x".repeat(LIMIT / 2 + 1))],
            ..Default::default()
        };
        assert!(vars.resolver().unwrap().text("{{a}}{{a}}").is_err());
        assert_eq!(
            vars.resolver().unwrap().json("{\n  \"id\": 1\n}").unwrap(),
            "{\n  \"id\": 1\n}"
        );
        assert!(vars.resolver().unwrap().text("{{a").is_err());
    }
    #[test]
    fn repeated_empty_expansions_cannot_exhaust_the_ui_thread() {
        let vars = Variables {
            globals: vec![
                variable("empty", ""),
                variable("fanout", &"{{empty}}".repeat(1000)),
            ],
            ..Default::default()
        };
        assert!(
            vars.resolver()
                .unwrap()
                .text(&"{{fanout}}".repeat(10))
                .is_err()
        );
    }
}
