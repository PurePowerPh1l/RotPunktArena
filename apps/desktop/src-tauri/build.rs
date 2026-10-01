fn main() {
    tauri_build::build();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Tauri mock IPC tests link TaskDialogIndirect through the runtime.
        // Unit/integration test executables need Common Controls v6 as well.
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
        // Production/lab binaries already receive Tauri's resource.lib manifest.
        // Disable only linker-generated manifests there to avoid duplicate RT_MANIFEST.
        println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
    }
}
