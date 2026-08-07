fn main() {
    #[cfg(target_os = "windows")]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
        let icon = std::path::Path::new(&manifest_dir)
            .join("../../icons/AppIcon.ico");
        winresource::WindowsResource::new()
            .set_icon(icon.to_str().expect("icon path is valid UTF-8"))
            .compile()
            .expect("failed to compile Windows resources");
    }
}
