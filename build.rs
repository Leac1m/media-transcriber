fn main() {
    slint_build::compile("ui/appwindow.slint").unwrap();

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("ui/icon.ico");
        res.compile().expect("failed to embed Windows resources");
    }
}
