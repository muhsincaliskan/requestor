use std::{env, fs, path::Path};

const WINDOWS_RESOURCES: &str = "requestor.rc";
const ICON_FILE: &str = "../../assets/icon/requestor.ico";
const VERSION_HEADER: &str = "version.h";

fn main() {
    println!("cargo:rerun-if-changed={WINDOWS_RESOURCES}");
    println!("cargo:rerun-if-changed={ICON_FILE}");
    write_version_header();
    embed_resource::compile(WINDOWS_RESOURCES, embed_resource::NONE)
        .manifest_optional()
        .expect("Windows resources compile");
}

fn write_version_header() {
    let header = format!(
        "#define REQUESTOR_VERSION \"{}\"\n#define REQUESTOR_VERSION_NUMBERS {},{},{},0\n",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH"),
    );
    let out_dir = env::var("OUT_DIR").expect("Cargo sets OUT_DIR for build scripts");
    fs::write(Path::new(&out_dir).join(VERSION_HEADER), header).expect("version header written");
}
