use std::{collections::BTreeMap, env, fs, io, path::PathBuf};

fn formats(text: &str) -> io::Result<Vec<String>> {
    let mut chars = text.chars().peekable();
    let mut fields = Vec::new();
    while let Some(c) = chars.next() {
        if c != '{' && c != '}' {
            continue;
        }
        if chars.peek() == Some(&c) {
            chars.next();
            continue;
        }
        if c == '}' {
            return Err(io::Error::other("Unmatched closing brace"));
        }
        let mut field = String::new();
        loop {
            match chars.next() {
                Some('}') => break,
                Some('{') | None => return Err(io::Error::other("Invalid format field")),
                Some(c) => field.push(c),
            }
        }
        fields.push(field);
    }
    fields.sort();
    Ok(fields)
}

fn translations() -> io::Result<()> {
    let languages = [
        ("zh-CN", "Chinese"),
        ("en", "English"),
        ("ja", "Japanese"),
        ("ko", "Korean"),
        ("fr", "French"),
        ("de", "German"),
        ("ru", "Russian"),
    ];
    let mut catalogs = Vec::new();
    for (code, _) in languages {
        let path = format!("locales/{code}.json");
        println!("cargo:rerun-if-changed={path}");
        let catalog: BTreeMap<String, String> = serde_json::from_str(&fs::read_to_string(&path)?)?;
        catalogs.push(catalog);
    }
    let base = &catalogs[1];
    for (index, catalog) in catalogs.iter().enumerate() {
        if !catalog.keys().eq(base.keys()) || catalog.values().any(|s| s.trim().is_empty()) {
            return Err(io::Error::other(format!(
                "Incomplete translation catalog: {}",
                languages[index].0
            )));
        }
    }
    println!("cargo:rerun-if-changed=locales/formats.json");
    let formatted: Vec<String> =
        serde_json::from_str(&fs::read_to_string("locales/formats.json")?)?;
    let mut output = String::from(
        "fn lookup(language: Language, message: &str) -> Option<&'static str> {\nmatch (language, message) {\n",
    );
    for ((_, variant), catalog) in languages.iter().zip(&catalogs) {
        for (key, value) in catalog {
            output.push_str(&format!(
                "(Language::{variant}, {key:?}) => Some({value:?}),\n"
            ));
        }
    }
    output.push_str("_ => None,\n}}\nmacro_rules! tr_format {\n");
    for key in formatted {
        let fields = formats(&key)?;
        output.push_str(&format!(
            "({key:?} $(, $($args:tt)*)?) => {{ match $crate::i18n::Language::current() {{\n"
        ));
        for ((code, variant), catalog) in languages.iter().zip(&catalogs) {
            let value = catalog
                .get(&key)
                .ok_or_else(|| io::Error::other(format!("Missing format: {key}")))?;
            if formats(value)? != fields {
                return Err(io::Error::other(format!(
                    "Format placeholders differ in {code}: {key}"
                )));
            }
            output.push_str(&format!(
                "$crate::i18n::Language::{variant} => format!({value:?} $(, $($args)*)?),\n"
            ));
        }
        output.push_str("}};\n");
    }
    output.push_str("}\n");
    let directory = env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("Missing OUT_DIR"))?;
    fs::write(PathBuf::from(directory).join("translations.rs"), output)
}

fn main() -> io::Result<()> {
    translations()?;
    println!("cargo:rerun-if-changed=assets/RPCRad.ico");
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("assets/RPCRad.ico")
        .set("ProductName", "RPCRad")
        .set("FileDescription", "RPCRad gRPC Workbench")
        .compile()?;
    Ok(())
}
