fn main() {
    println!("cargo:rerun-if-changed=icons/icon.rc");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("icons/icon.rc", embed_resource::NONE)
            .manifest_required()
            .expect("Failed to embed the Latch application icon");
    }
}
