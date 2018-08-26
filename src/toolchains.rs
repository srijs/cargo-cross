use std::ffi::OsString;
use std::path::PathBuf;

use directories::ProjectDirs;
use failure::Error;
use heck::ShoutySnakeCase;
use platforms;
use semver::VersionReq;

use cargo::{CargoPackage, CargoProject};
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

    pub fn is_toolchain_feature_available(&self, target: &str, cargo_pkg: &CargoPackage) -> bool {
        self.find_toolchain_feature(target, cargo_pkg).is_some()
    }

    pub fn is_toolchain_feature_installed(&self, target: &str, cargo_pkg: &CargoPackage) -> bool {
        self.find_toolchain_feature(target, cargo_pkg)
            .map(|feature| self.get_toolchain_feature_path(feature).exists())
            .unwrap_or(false)
    }

    pub fn start_toolchain_base_installation(&self, target: &str) -> Result<PackageInstall, Error> {
        let base = self.find_toolchain_base(target)
            .ok_or_else(|| format_err!("no toolchain available for target {}", target))?;
        let path = self.get_toolchain_base_path(&base);
        self.package_manager.install(base.path, base.size, path)
    }

    pub fn start_toolchain_feature_installation(
        &self,
        target: &str,
        cargo_pkg: &CargoPackage,
    ) -> Result<PackageInstall, Error> {
        let feature = self.find_toolchain_feature(target, cargo_pkg)
            .ok_or_else(|| format_err!("toolchain feature not available for target {}", target))?;
        let path = self.get_toolchain_feature_path(&feature);
        self.package_manager
            .install(feature.path, feature.size, path)
    }

    pub fn get_toolchain_environment(
        &self,
        target: &str,
        project: &CargoProject,
    ) -> Result<impl IntoIterator<Item = (String, OsString)>, Error> {
        let base = self.find_toolchain_base(target)
            .ok_or_else(|| format_err!("no toolchain available for target {}", target))?;
        let path = self.get_toolchain_base_path(&base);

        let gcc_path = path.join("bin").join(format!("{}-gcc", target));
        let gcc_include_path = path.join("lib")
            .join("gcc")
            .join(target)
            .join(base.gcc_version)
            .join("include");
        let gcc_include_fixed_path = path.join("lib")
            .join("gcc")
            .join(target)
            .join(base.gcc_version)
            .join("include-fixed");

        let mut cflags = OsString::from("");
        cflags.push(" -I ");
        cflags.push(&gcc_include_path);
        cflags.push(" -I ");
        cflags.push(&gcc_include_fixed_path);

        let mut envs = vec![
            (
                format!("CARGO_TARGET_{}_LINKER", target.to_shouty_snake_case()),
                gcc_path.clone().into_os_string(),
            ),
            ("TARGET_CC".into(), gcc_path.clone().into_os_string()),
            ("TARGET_CFLAGS".into(), cflags),
            ("CHOST".into(), target.into()),
        ];

        for cargo_pkg in project.packages.iter() {
            if let Some(feature) = self.find_toolchain_feature(target, &cargo_pkg) {
                let feature_path = self.get_toolchain_feature_path(&feature);
                for (k, v) in feature.env_vars {
                    envs.push((
                        (*k).into(),
                        v.replace("{CARGO_CROSS_FEAT_PATH}", &feature_path.to_string_lossy())
                            .into(),
                    ));
                }
            }
        }

        Ok(envs)
    }

    fn find_toolchain_base(&self, target: &str) -> Option<&ToolchainBase> {
        TOOLCHAINS_BASE.iter().find(|t| {
            t.target_platform_triple == target && t.host_platform_triple == self.host.target_triple
        })
    }

    fn find_toolchain_feature(
        &self,
        target: &str,
        cargo_pkg: &CargoPackage,
    ) -> Option<&ToolchainFeature> {
        TOOLCHAIN_FEATURES
            .iter()
            .filter(|t| t.target_platform_triple == target && t.crate_name == cargo_pkg.name)
            .find(|t| {
                let vreq =
                    VersionReq::parse(t.crate_version_req).expect("failed to parse version req");
                vreq.matches(&cargo_pkg.version)
            })
    }

    fn get_toolchain_base_path(&self, base: &ToolchainBase) -> PathBuf {
        let mut dir = self.dirs.cache_dir().to_path_buf();
        dir.extend(&[
            "target",
            base.target_platform_triple,
            "base",
            &base.checksum,
        ]);
        dir
    }

    fn get_toolchain_feature_path(&self, feature: &ToolchainFeature) -> PathBuf {
        let mut dir = self.dirs.cache_dir().to_path_buf();
        dir.extend(&[
            "target",
            feature.target_platform_triple,
            "feature",
            &feature.checksum,
        ]);
        dir
    }
}

struct ToolchainBase {
    target_platform_triple: &'static str,
    host_platform_triple: &'static str,
    gcc_version: &'static str,
    path: &'static str,
    checksum: &'static str,
    size: u64,
}

static TOOLCHAIN_MIRROR: &str = "https://d3ojaw7tkwhzj5.cloudfront.net/";

static TOOLCHAINS_BASE: &[ToolchainBase] = &[ToolchainBase {
    host_platform_triple: "x86_64-apple-darwin",
    target_platform_triple: "x86_64-unknown-linux-gnu",
    gcc_version: "4.8.5",
    path: "target/x86_64-unknown-linux-gnu/base-x86_64-apple-darwin-4de47685.tar.xz",
    size: 26366476,
    checksum: "4de47685",
}];

struct ToolchainFeature {
    target_platform_triple: &'static str,
    crate_name: &'static str,
    crate_version_req: &'static str,
    path: &'static str,
    size: u64,
    checksum: &'static str,
    env_vars: &'static [(&'static str, &'static str)],
}

static TOOLCHAIN_FEATURES: &[ToolchainFeature] = &[ToolchainFeature {
    target_platform_triple: "x86_64-unknown-linux-gnu",
    crate_name: "openssl-sys",
    crate_version_req: "^0.9",
    path: "target/x86_64-unknown-linux-gnu/feat-openssl-1.0.2o-49b96e34.tar.xz",
    size: 1359060,
    checksum: "49b96e34",
    env_vars: &[
        ("OPENSSL_DIR", "{CARGO_CROSS_FEAT_PATH}"),
        ("OPENSSL_STATIC", "1"),
    ],
}];
