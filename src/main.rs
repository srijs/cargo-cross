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

mod package;
mod toolchains;
mod utils;
use self::toolchains::ToolchainManager;

#[derive(StructOpt)]
#[structopt(
    name = "cargo-cross",
    author = "",
    raw(
        global_settings = "&[AppSettings::UnifiedHelpMessage, AppSettings::VersionlessSubcommands]"
    )
)]
enum Command {
    #[structopt(
        name = "build",
        about = "Compile a local package and all of its dependencies",
        author = "",
        version = ""
    )]
    Build(CommandBuildOpts),
}

#[derive(StructOpt)]
struct CommandBuildOpts {
    #[structopt(name = "TARGET", long = "target")]
    target: String,
}

fn main() {
    env_logger::init();

    let dirs =
        ProjectDirs::from("", "", "cargo-cross").expect("could not determine project directories");

    if let Err(err) = command(dirs) {
        eprintln!("{} {}", style("error:").red().bold(), err);
        process::exit(1);
    }
}

fn command(dirs: ProjectDirs) -> Result<(), Error> {
    match Command::from_args() {
        Command::Build(opts) => command_build(dirs, opts),
    }
}

fn command_build(dirs: ProjectDirs, opts: CommandBuildOpts) -> Result<(), Error> {
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
        install_toolchain_base(&manager, &opts.target)?;
    }

    let mut build_command = process::Command::new("cargo");
    build_command.args(&["build", "--target", &opts.target]);
    build_command.envs(manager.get_toolchain_environment(&opts.target)?);

    let status = build_command.status()?;

    if !status.success() {
        process::exit(1);
    }

    Ok(())
}

fn install_toolchain_base(manager: &ToolchainManager, target: &str) -> Result<(), Error> {
    let mut install = manager
        .start_toolchain_installation(target)
        .map_err(|err| format_err!("failed to start toolchain install: {}", err))?;

    let progress_bar = ProgressBar::new(install.total());
    progress_bar.set_message("Fetch");
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{msg:>12.cyan.bold} {bytes} / {total_bytes} [{wide_bar}] {percent}%  ")
            .progress_chars("=>-"),
    );

    while let Some(progress) = install.wait_progress() {
        progress_bar.set_position(progress);
    }

    install
        .wait_complete()
        .map_err(|err| format_err!("could not complete toolchain installation: {}", err))?;

    progress_bar.finish_and_clear();

    Ok(())
}
