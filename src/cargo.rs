use std::ffi::OsStr;
use std::process;

use failure::Error;
use semver::Version;

#[derive(Deserialize)]
pub struct CargoProject {
    pub packages: Vec<CargoPackage>,
}

#[derive(Deserialize)]
pub struct CargoPackage {
    pub name: String,
    pub version: Version,
}

#[derive(StructOpt)]
pub struct CargoOptions {
    #[structopt(value_name = "TRIPLE", long = "target", help = "Build for the target triple")]
    pub target: String,
    #[structopt(value_name = "SPEC", short = "p", long = "package", help = "Package to build")]
    pub package: Option<String>,
    #[structopt(long = "all", help = "Build all packages in the workspace")]
    pub all: bool,
    #[structopt(long = "lib", help = "Build only this package's library")]
    pub lib: bool,
    #[structopt(value_name = "NAME", long = "bin", help = "Build only the specified binary")]
    pub bin: Option<String>,
    #[structopt(long = "libs", help = "Build all binaries")]
    pub bins: bool,
    #[structopt(value_name = "NAME", long = "example", help = "Build only the specified example")]
    pub example: Option<String>,
    #[structopt(long = "examples", help = "Build all examples")]
    pub examples: bool,
    #[structopt(long = "release", help = "Build artifacts in release mode, with optimizations")]
    pub release: bool,
    #[structopt(short = "v", long = "verbose", help = "Use verbose output", parse(from_occurrences))]
    pub verbose: u64,
}

impl CargoOptions {
    fn apply_all(&self, command: &mut process::Command) {
        command.args(&["--target", &self.target]);
        if let Some(ref package) = self.package {
            command.args(&["--package", package]);
        }
        if self.all {
            command.arg("--all");
        }
        if self.lib {
            command.arg("--lib");
        }
        if let Some(ref bin) = self.bin {
            command.args(&["--bin", bin]);
        }
        if self.bins {
            command.arg("--bins");
        }
        if let Some(ref example) = self.example {
            command.args(&["--example", example]);
        }
        if self.examples {
            command.arg("--examples");
        }
        if self.release {
            command.arg("--release");
        }
        self.apply_verbose(command);
    }

    fn apply_verbose(&self, command: &mut process::Command) {
        if self.verbose == 1 {
            command.arg("-v");
        } else if self.verbose > 1 {
            command.arg("-vv");
        }
    }
}

pub fn build<I, K, V>(opts: &CargoOptions, envs: I) -> Result<process::ExitStatus, Error>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let mut command = process::Command::new("cargo");
    command.arg("build");
    opts.apply_all(&mut command);
    command.envs(envs);
    Ok(command.status()?)
}

pub fn metadata(_opts: &CargoOptions) -> Result<CargoProject, Error> {
    let mut command = process::Command::new("cargo");
    command.args(&["metadata", "-q", "--format-version", "1"]);

    let output = command.output()?;

    if !output.status.success() {
        bail!("Could not retrieve project metadata.");
    }

    Ok(::serde_json::from_slice(&output.stdout)?)
}
