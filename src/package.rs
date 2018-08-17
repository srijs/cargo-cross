use std::fs;
use std::path::PathBuf;
use std::thread;

use bzip2::read::BzDecoder;
use failure::Error;
use reqwest::{Client, Url};
use tar::Archive;
use tempfile;

use utils::read_progress;

#[derive(Debug)]
pub struct PackageManager {
    base_url: Url,
    client: Client,
}

impl PackageManager {
    pub fn new(base_url: &str) -> Result<Self, Error> {
        let parsed_base_url = base_url.parse()?;
        let client = Client::new();
        Ok(PackageManager {
            base_url: parsed_base_url,
            client,
        })
    }

    pub fn install(
        &self,
        remote_path: &str,
        total_size: u64,
        local_path: PathBuf,
    ) -> Result<PackageInstall, Error> {
        debug!(
            "install {} {} {}",
            remote_path,
            total_size,
            local_path.display()
        );
        let url = self.base_url.join(remote_path)?;
        let client = self.client.clone();
        let temp_dir = tempfile::tempdir()?;
        let temp_dir_path = temp_dir.as_ref().to_path_buf();
        debug!("temp dir {}", temp_dir_path.display());
        let response = client.get(url).send()?.error_for_status()?;
        let (read, progress_signal) = read_progress::read_progress(response);
        let join_handle = thread::spawn(move || {
            let bunzip = BzDecoder::new(read);
            let mut archive = Archive::new(bunzip);
            archive.unpack(&temp_dir_path)?;
            fs::create_dir_all(local_path.parent().unwrap())?;
            Ok(fs::rename(temp_dir_path, local_path)?)
        });

        Ok(PackageInstall {
            total_size,
            progress_signal,
            temp_dir,
            join_handle,
        })
    }
}

pub struct PackageInstall {
    total_size: u64,
    progress_signal: read_progress::ReadProgressSignal,
    temp_dir: tempfile::TempDir,
    join_handle: thread::JoinHandle<Result<(), Error>>,
}

impl PackageInstall {
    pub fn total(&self) -> u64 {
        self.total_size
    }

    pub fn wait_progress(&mut self) -> Option<u64> {
        self.progress_signal.next()
    }

    pub fn wait_complete(self) -> Result<(), Error> {
        self.join_handle.join().unwrap()?;
        drop(self.temp_dir);
        Ok(())
    }
}
