fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=assets/RPCRad.ico");
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("assets/RPCRad.ico")
        .set("ProductName", "RPCRad")
        .set("FileDescription", "RPCRad gRPC Workbench")
        .compile()?;
    Ok(())
}
