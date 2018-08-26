use std::fs;
use std::path::PathBuf;

use failure::Error;
use reqwest::{Client, Url};
use tar::Archive;
use tempfile;
use xz2::read::XzDecoder;

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
        Ok(PackageInstall {
            total_size,
            client,
            url,
            local_path,
        })
    }
}

pub struct PackageInstall {
    total_size: u64,
    client: Client,
    url: Url,
    local_path: PathBuf,
}

impl PackageInstall {
    pub fn total(&self) -> u64 {
        self.total_size
    }

    pub fn perform<P>(self, observer: P) -> Result<(), Error>
    where
        P: progress::ProgressObserver + Send + 'static,
    {
        let temp_dir = tempfile::tempdir()?;
        debug!("temp dir {:?}", temp_dir);

        let response = self.client
            .get(self.url.clone())
            .send()?
            .error_for_status()?;
        let read = observer.observe_read(response);
        let bunzip = XzDecoder::new(read);
        let mut archive = Archive::new(bunzip);
        archive.unpack(&temp_dir)?;

        fs::create_dir_all(self.local_path.parent().unwrap())?;

        if let Err(err) = fs::rename(temp_dir, &self.local_path) {
            // a concurrent process might have installed the package,
            // so we check if the path exists
            if !self.local_path.exists() {
                return Err(err.into());
            }
        }

        Ok(())
    }
}
