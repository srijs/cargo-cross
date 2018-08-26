use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use chttp::Client;
use failure::Error;
use tar::Archive;
use tempfile;
use xz2::read::XzDecoder;

use utils::hasher;
use utils::progress;

pub struct PackageManager {
    base_url: String,
    client: Rc<Client>,
}

impl PackageManager {
    pub fn new(base_url: &str) -> Result<Self, Error> {
        let client = Client::new();
        Ok(PackageManager {
            base_url: base_url.to_owned(),
            client: Rc::new(client),
        })
    }

    pub fn install(
        &self,
        remote_path: &str,
        total_size: u64,
        checksum: &'static str,
        local_path: PathBuf,
    ) -> Result<PackageInstall, Error> {
        debug!("install {}", remote_path);
        let url = format!("{}/{}", self.base_url, remote_path);
        let client = self.client.clone();
        Ok(PackageInstall {
            total_size,
            checksum,
            client,
            url,
            local_path,
        })
    }
}

pub struct PackageInstall {
    total_size: u64,
    checksum: &'static str,
    client: Rc<Client>,
    url: String,
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

        let response = self.client.get(&self.url)?;
        if !response.status().is_success() {
            bail!("unexpected status code {}", response.status());
        }

        let read = observer.observe_read(response.into_body());
        let hash = hasher::ReadHasher::new(read);
        let bunzip = XzDecoder::new(hash);
        let mut archive = Archive::new(bunzip);
        archive.unpack(&temp_dir)?;

        let digest = archive.into_inner().into_inner().digest_hex();
        if digest != self.checksum {
            bail!(
                "checksum mismatch (expected {}, got {})",
                self.checksum,
                digest
            );
        }

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
