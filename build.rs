use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=logo.png");
    println!("cargo:rerun-if-changed=cpp/drag.cpp");
    println!("cargo:rerun-if-changed=cpp/drag.h");
    println!("cargo:rustc-check-cfg=cfg(has_cpp_backend)");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR should be set"));
    let icon_path = out_dir.join("3-win-drag.ico");

    generate_icon(&icon_path);

    println!(
        "cargo:rustc-env=THREE_WIN_DRAG_ICON_PATH={}",
        icon_path.display()
    );

    compile_windows_resources(&out_dir, &icon_path);

    if compiler_available() {
        cc::Build::new()
            .cpp(true)
            .std("c++17")
            .warnings(true)
            .file("cpp/drag.cpp")
            .include("cpp")
            .compile("drag_native");
        println!("cargo:rustc-cfg=has_cpp_backend");
    } else {
        println!(
            "cargo:warning=MSVC-compatible C++ compiler was not found. Building with the Rust Windows backend fallback."
        );
    }
}

fn compiler_available() -> bool {
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    env::var_os("CXX").is_some()
        || command_exists("cl.exe")
        || (target_env == "gnu" && (command_exists("g++.exe") || command_exists("c++.exe")))
}

fn command_exists(command: &str) -> bool {
    Command::new("where")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn generate_icon(target_path: &Path) {
    let img = image::open("logo.png")
        .expect("logo.png must be available for icon generation")
        .resize(256, 256, image::imageops::FilterType::Lanczos3)
        .into_rgba8();

    img.save(target_path)
        .expect("failed to generate .ico from logo.png");
}

fn compile_windows_resources(out_dir: &Path, icon_path: &Path) {
    let resource_path = out_dir.join("3-win-drag.rc");
    let resource_body = format!("APPICON ICON \"{}\"\n", icon_path.display());
    fs::write(&resource_path, resource_body).expect("failed to write Windows resource script");
    let _ = embed_resource::compile(resource_path, embed_resource::NONE);
}
