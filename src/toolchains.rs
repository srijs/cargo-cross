use std::ffi::OsString;
use std::path::PathBuf;

use directories::ProjectDirs;
use failure::Error;
use heck::ShoutySnakeCase;
use platforms;

use package::{PackageInstall, PackageManager};

#[derive(Debug)]
pub struct ToolchainManager {
    dirs: ProjectDirs,
    host: platforms::Platform,
    package_manager: PackageManager,
}

pub struct ToolchainInfo {
    pub gcc_version: &'static str,
}

impl ToolchainManager {
    pub fn new(dirs: &ProjectDirs) -> ToolchainManager {
        let host = platforms::guess_current().expect("unknown toolchain host");

        let package_manager =
            PackageManager::new(TOOLCHAIN_MIRROR).expect("could not initialize package manager");

        ToolchainManager {
            dirs: dirs.clone(),
            host: host.clone(),
            package_manager,
        }
    }

    pub fn host(&self) -> &str {
        self.host.target_triple
    }

    pub fn get_toolchain_info(&self, target: &str) -> Option<ToolchainInfo> {
        self.find_toolchain_base(target).map(|base| ToolchainInfo {
            gcc_version: base.gcc_version,
        })
    }

    pub fn is_toolchain_base_available(&self, target: &str) -> bool {
        self.find_toolchain_base(target).is_some()
    }

    pub fn is_toolchain_base_installed(&self, target: &str) -> bool {
        self.find_toolchain_base(target)
            .map(|base| self.get_toolchain_base_path(base).exists())
            .unwrap_or(false)
    }

    pub fn start_toolchain_installation(&self, target: &str) -> Result<PackageInstall, Error> {
        let base = self.find_toolchain_base(target)
            .ok_or_else(|| format_err!("no toolchain available for target {}", target))?;
        let path = self.get_toolchain_base_path(&base);
        self.package_manager.install(base.path, base.size, path)
    }

    pub fn get_toolchain_environment(
        &self,
        target: &str,
    ) -> Result<impl IntoIterator<Item = (String, OsString)>, Error> {
        let base = self.find_toolchain_base(target)
            .ok_or_else(|| format_err!("no toolchain available for target {}", target))?;
        let path = self.get_toolchain_base_path(&base);

        let gcc_path = path.join("bin").join(format!("{}-gcc", target));
        let include_path = path.join(target).join("include");
        let gcc_include_path = path.join("lib")
            .join("gcc")
            .join(target)
            .join(base.gcc_version)
            .join("include");

        let mut cflags = OsString::from("-nostdinc");
        cflags.push(" -I ");
        cflags.push(&include_path);
        cflags.push(" -I ");
        cflags.push(&gcc_include_path);
        cflags.push(" -isystem ");
        cflags.push(&include_path);
        cflags.push(" --sysroot ");
        cflags.push(&path);

        Ok(vec![
            (
                format!("CARGO_TARGET_{}_LINKER", target.to_shouty_snake_case()),
                gcc_path.clone().into_os_string(),
            ),
            ("TARGET_CC".into(), gcc_path.clone().into_os_string()),
            ("TARGET_CFLAGS".into(), cflags),
        ])
    }

    fn find_toolchain_base(&self, target: &str) -> Option<&ToolchainBase> {
        TOOLCHAINS_BASE.iter().find(|t| {
            t.target_platform_triple == target && t.host_platform_triple == self.host.target_triple
        })
    }

    fn get_toolchain_base_path(&self, base: &ToolchainBase) -> PathBuf {
        let mut dir = self.dirs.cache_dir().to_path_buf();
        dir.extend(&[
            "target",
            base.target_platform_triple,
            "base",
            &base.checksum[..10],
        ]);
        dir
    }
}

struct ToolchainBase {
    target_platform_triple: &'static str,
    host_platform_triple: &'static str,
    gcc_version: &'static str,
    path: &'static str,
    size: u64,
    checksum: &'static str,
}

static TOOLCHAIN_MIRROR: &str = "https://d3ojaw7tkwhzj5.cloudfront.net/";

static TOOLCHAINS_BASE: &[ToolchainBase] = &[ToolchainBase {
    host_platform_triple: "x86_64-apple-darwin",
    target_platform_triple: "x86_64-unknown-linux-gnu",
    gcc_version: "4.8.5",
    path: "target/x86_64-unknown-linux-gnu/base-x86_64-apple-darwin-36f6e7a0.tar.bz2",
    size: 74774494,
    checksum: "5280e4a4bf8446da89bdddeea3f891cc9feb1681e8bfdb35317e99617746dd0e",
}];
