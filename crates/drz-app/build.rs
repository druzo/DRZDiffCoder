fn main() {
    #[cfg(target_os = "windows")]
    {
        // CARGO_MANIFEST_DIR is crates/drz-app; icons/ is at the workspace root.
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
        let icon = std::path::Path::new(&manifest_dir).join("../../icons/AppIcon.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon(icon.to_str().expect("icon path utf-8"));
        res.compile().expect("failed to compile Windows resources");
    }
}
