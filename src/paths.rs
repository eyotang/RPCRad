use std::path::{Path, PathBuf};

pub fn data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("RPCRAD_DATA_DIR") {
        return PathBuf::from(path);
    }
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("RPCRad")
}
pub fn state_path() -> PathBuf {
    data_dir().join("workspace.json")
}
pub fn legacy_state_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".rpcrad/workspace.json")
}
pub fn resources() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let macos = exe.parent()?;
    if macos.file_name()? != "MacOS" {
        return None;
    }
    Some(macos.parent()?.join("Resources"))
}
pub fn examples() -> Vec<PathBuf> {
    let root = resources()
        .map(|p| p.join("protos"))
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/protos"));
    ["user.proto", "order.proto", "health.proto"]
        .iter()
        .map(|name| root.join(name))
        .collect()
}
// Persist bundled example references independently of where the app is installed.
pub fn save_proto(path: &Path) -> PathBuf {
    if examples().contains(&path.to_path_buf())
        && let Some(name) = path.file_name()
    {
        return PathBuf::from("rpcrad-examples").join(name);
    }
    path.to_path_buf()
}
pub fn load_proto(path: &Path) -> PathBuf {
    let legacy = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/protos");
    let bundled = path
        .parent()
        .is_some_and(|p| p.ends_with("RPCRad.app/Contents/Resources/protos"));
    if (path.parent() == Some(Path::new("rpcrad-examples"))
        || path.parent() == Some(legacy.as_path())
        || bundled)
        && let Some(name) = path.file_name()
        && let Some(example) = examples().into_iter().find(|p| p.file_name() == Some(name))
    {
        return example;
    }
    path.to_path_buf()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_examples_survive_application_relocation() {
        for path in examples() {
            assert_eq!(load_proto(&save_proto(&path)), path);
        }
        assert_eq!(
            load_proto(Path::new(
                "/old/RPCRad.app/Contents/Resources/protos/user.proto"
            )),
            examples()[0]
        );
    }
    #[test]
    fn custom_proto_paths_are_not_replaced() {
        let path = Path::new("/custom/apis/user.proto");
        assert_eq!(load_proto(path), path);
        assert_eq!(save_proto(path), path);
    }
}
