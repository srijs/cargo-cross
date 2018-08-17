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
    #[structopt(name = "TARGET", long = "target")]
    pub target: String,
}

pub fn build<I, K, V>(opts: &CargoOptions, envs: I) -> Result<process::ExitStatus, Error>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let mut command = process::Command::new("cargo");
    command.args(&["build", "--target", &opts.target]);
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
