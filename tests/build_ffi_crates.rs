extern crate assert_cmd;
extern crate cargo_toml_builder;
extern crate heck;
extern crate tempfile;

use std::fs;
use std::process::Command;

use assert_cmd::prelude::*;
use cargo_toml_builder::prelude::*;
use heck::SnakeCase;
use tempfile::TempDir;

fn compile_test(crate_name: &str, crate_version: &str, target: &str, main: &str) {
    let project_dir = TempDir::new().unwrap();

    let toml = CargoToml::builder()
        .name("compile-test")
        .author("me")
        .dependency(crate_name.version(crate_version))
        .build()
        .unwrap();

    let main = format!(
        "extern crate {};\n\nfn main() {{ {} }}\n",
        crate_name.to_snake_case(),
        main
    );

    fs::write(project_dir.as_ref().join("Cargo.toml"), toml.to_string()).unwrap();
    fs::create_dir(project_dir.as_ref().join("src")).unwrap();
    fs::write(project_dir.as_ref().join("src/main.rs"), main).unwrap();

    let mut cmd = Command::main_binary().unwrap();
    cmd.current_dir(&project_dir);
    cmd.args(&["cross", "build", "--target", target]);

    cmd.assert().success();
}

#[test]
fn openssl_sys_0_9_35_for_x86_64_unknown_linux_gnu() {
    compile_test(
        "openssl-sys",
        "0.9.35",
        "x86_64-unknown-linux-gnu",
        "::openssl_sys::init();",
    );
}

#[test]
fn miniz_sys_0_1_10_for_x86_64_unknown_linux_gnu() {
    compile_test(
        "miniz-sys",
        "0.1.10",
        "x86_64-unknown-linux-gnu",
        "unsafe { ::miniz_sys::mz_crc32(0, ::std::ptr::null(), 0); }",
    );
}

#[test]
fn libz_sys_1_0_20_for_x86_64_unknown_linux_gnu() {
    compile_test(
        "libz-sys",
        "1.0.20",
        "x86_64-unknown-linux-gnu",
        "unsafe { ::libz_sys::zlibVersion(); }",
    );
}

#[test]
fn lzma_sys_0_1_10_for_x86_64_unknown_linux_gnu() {
    compile_test(
        "lzma-sys",
        "0.1.10",
        "x86_64-unknown-linux-gnu",
        "unsafe { ::lzma_sys::lzma_version_number(); }",
    );
}

#[test]
fn bzip2_sys_0_1_6_for_x86_64_unknown_linux_gnu() {
    compile_test("bzip2-sys", "0.1.6", "x86_64-unknown-linux-gnu", "");
}

#[test]
fn libsqlite3_sys_0_9_3_for_x86_64_unknown_linux_gnu() {
    compile_test(
        "libsqlite3-sys",
        "0.9.3",
        "x86_64-unknown-linux-gnu",
        "unsafe { ::libsqlite3_sys::sqlite3_libversion(); }",
    );
}

#[test]
fn brotli_sys_0_3_2_for_x86_64_unknown_linux_gnu() {
    compile_test(
        "brotli-sys",
        "0.3.2",
        "x86_64-unknown-linux-gnu",
        "unsafe { ::brotli_sys::BrotliEncoderVersion(); }",
    );
}
