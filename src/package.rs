use std::fs;
use std::path::PathBuf;
use std::thread;

use bzip2::read::BzDecoder;
use failure::Error;
use reqwest::{Client, Url};
use tar::Archive;
use tempfile;

use utils::progress;

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
        debug!("install {}", remote_path);
        let url = self.base_url.join(remote_path)?;
        let client = self.client.clone();
        let temp_dir = tempfile::tempdir()?;
        debug!("temp dir {:?}", temp_dir);

        Ok(PackageInstall {
            total_size,
            client,
            url,
            local_path,
            temp_dir,
            join_handle: None,
        })
    }
}

pub struct PackageInstall {
    total_size: u64,
    client: Client,
    url: Url,
    temp_dir: tempfile::TempDir,
    local_path: PathBuf,
    join_handle: Option<thread::JoinHandle<Result<(), Error>>>,
}

impl PackageInstall {
    pub fn start<P>(&mut self, observer: P) -> Result<(), Error>
    where
        P: progress::ProgressObserver + Send + 'static,
    {
        let response = self.client
            .get(self.url.clone())
            .send()?
            .error_for_status()?;
        let read = observer.observe_read(response);
        let temp_dir_path = self.temp_dir.as_ref().to_path_buf();
        self.join_handle = Some(thread::spawn(move || {
            let bunzip = BzDecoder::new(read);
            let mut archive = Archive::new(bunzip);
            Ok(archive.unpack(temp_dir_path)?)
        }));
        Ok(())
    }

    pub fn total(&self) -> u64 {
        self.total_size
    }

    pub fn wait(self) -> Result<(), Error> {
        self.join_handle.unwrap().join().unwrap()?;
        fs::create_dir_all(self.local_path.parent().unwrap())?;
        Ok(fs::rename(self.temp_dir, self.local_path)?)
    }
}
