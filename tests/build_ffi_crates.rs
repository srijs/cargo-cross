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

fn compile_test(crate_name: &str, crate_version: &str, target: &str) {
    let project_dir = TempDir::new().unwrap();

    let toml = CargoToml::builder()
        .name("compile-test")
        .author("me")
        .dependency(crate_name.version(crate_version))
        .build()
        .unwrap();

    let main = format!(
        "extern crate {};\n\nfn main() {{}}\n",
        crate_name.to_snake_case()
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
fn openssl_sys_for_x86_64_unknown_linux_gnu() {
    compile_test("openssl-sys", "0.9.35", "x86_64-unknown-linux-gnu");
}

#[test]
fn lzma_sys_for_x86_64_unknown_linux_gnu() {
    compile_test("lzma-sys", "0.1.10", "x86_64-unknown-linux-gnu");
}
