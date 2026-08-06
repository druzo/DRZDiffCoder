fn main() {
    #[cfg(target_os = "windows")]
    {
        winresource::WindowsResource::new()
            .set_icon("icons/AppIcon.ico")
            .expect("failed to set icon")
            .compile()
            .expect("failed to compile Windows resources");
    }
}
