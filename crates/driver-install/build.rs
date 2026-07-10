fn main() {
    #[cfg(target_os = "windows")]
    embed_resource::compile_for_everything("tests/tests.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
