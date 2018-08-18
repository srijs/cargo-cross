extern crate bzip2;
extern crate console;
extern crate directories;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate heck;
extern crate indicatif;
#[macro_use]
extern crate log;
extern crate pbr;
extern crate platforms;
extern crate reqwest;
extern crate semver;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate structopt;
extern crate tar;
extern crate tempfile;

use std::process;

use console::style;
use directories::ProjectDirs;
use failure::Error;
use indicatif::{ProgressBar, ProgressStyle};
use structopt::clap::AppSettings;
use structopt::StructOpt;

mod cargo;
mod package;
mod toolchains;
mod utils;

use self::cargo::CargoOptions;
use self::package::PackageInstall;
use self::toolchains::ToolchainManager;
use self::utils::progress::ProgressObserver;

#[derive(StructOpt)]
#[structopt(
    name = "cargo",
    author = "",
    bin_name = "cargo",
    raw(global_settings = "&[AppSettings::UnifiedHelpMessage]")
)]
enum Cargo {
    #[structopt(
        name = "cross", author = "", raw(global_settings = "&[AppSettings::VersionlessSubcommands]")
    )]
    Cross(Command),
}

#[derive(StructOpt)]
enum Command {
    #[structopt(
        name = "build",
        about = "Compile a local package and all of its dependencies",
        author = "",
        version = ""
    )]
    Build(CargoOptions),
}

fn main() {
    env_logger::init();

    let dirs =
        ProjectDirs::from("", "", "cargo-cross").expect("could not determine project directories");

    let Cargo::Cross(cmd) = Cargo::from_args();

    if let Err(err) = command(dirs, cmd) {
        eprintln!("{} {}", style("error:").red().bold(), err);
        process::exit(1);
    }
}

fn command(dirs: ProjectDirs, cmd: Command) -> Result<(), Error> {
    match cmd {
        Command::Build(opts) => command_build(dirs, opts),
    }
}

fn command_build(dirs: ProjectDirs, opts: CargoOptions) -> Result<(), Error> {
    let manager = ToolchainManager::new(&dirs);

    let info = manager.get_toolchain_info(&opts.target).ok_or_else(|| {
        format_err!(
            "Cross compilation for target {} not supported.",
            opts.target
        )
    })?;

    if !manager.is_toolchain_base_available(&opts.target) {
        bail!(
            "Could not find suitable toolchain for selected target ({}) and host ({}) system.",
            opts.target,
            manager.host(),
        );
    }

    println!(
        "{:>12} {} (gcc {})",
        style("Toolchain").magenta().bold(),
        opts.target,
        info.gcc_version
    );

    if !manager.is_toolchain_base_installed(&opts.target) {
        let install = manager.start_toolchain_base_installation(&opts.target)?;
        package_install_progress(install)?;
    }

    let metadata = cargo::metadata(&opts)?;

    for package in metadata.packages.iter() {
        if manager.is_toolchain_feature_available(&opts.target, &package) {
            println!(
                "{:>12} {} v{}",
                style("Support").magenta().bold(),
                package.name,
                package.version
            );
            if !manager.is_toolchain_feature_installed(&opts.target, &package) {
                let install = manager.start_toolchain_feature_installation(&opts.target, &package)?;
                package_install_progress(install)?;
            }
        }
    }

    let env = manager.get_toolchain_environment(&opts.target, &metadata)?;
    let status = cargo::build(&opts, env)?;
    if !status.success() {
        process::exit(1);
    }

    Ok(())
}

fn package_install_progress(mut install: PackageInstall) -> Result<(), Error> {
    let progress_bar = ProgressBar::new(install.total());

    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg:>12.cyan.bold} {bytes} / {total_bytes} [{wide_bar}] {percent}%  ")
            .progress_chars("=>-"),
    );
    progress_bar.set_message("Fetch");

    struct ProgressBarObserver(ProgressBar);

    impl ProgressObserver for ProgressBarObserver {
        fn progress(&mut self, delta: u64) {
            self.0.inc(delta);
        }

        fn complete(&mut self) {
            self.0.finish_and_clear();
        }
    }

    install.start(ProgressBarObserver(progress_bar))?;
    install.wait()?;

    Ok(())
}
