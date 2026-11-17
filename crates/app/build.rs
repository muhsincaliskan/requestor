const WINDOWS_RESOURCES: &str = "requestor.rc";
const ICON_FILE: &str = "../../assets/icon/requestor.ico";

fn main() {
    println!("cargo:rerun-if-changed={WINDOWS_RESOURCES}");
    println!("cargo:rerun-if-changed={ICON_FILE}");
    embed_resource::compile(WINDOWS_RESOURCES, embed_resource::NONE)
        .manifest_optional()
        .expect("Windows resources compile");
}
