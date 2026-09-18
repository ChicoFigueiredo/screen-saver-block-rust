#[cfg(windows)]
fn main() {
    let mut resources = winres::WindowsResource::new();
    resources.set_icon("assets/preferences-desktop-screensaver.ico");
    resources.set("FileDescription", "Block Screen Saver");
    resources.set("ProductName", "Block Screen Saver");
    resources.set("OriginalFilename", "block-screen-saver.exe");
    resources
        .compile()
        .expect("não foi possível incorporar o ícone Windows");

    println!("cargo:rerun-if-changed=assets/preferences-desktop-screensaver.ico");
}

#[cfg(not(windows))]
fn main() {}
