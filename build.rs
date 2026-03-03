fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("DisplayWarp.ico");

        if let Ok(version) = std::env::var("CARGO_PKG_VERSION") {
            res.set("FileVersion", &version);
            res.set("ProductVersion", &version);
        }

        res.compile().unwrap();
    }
}
