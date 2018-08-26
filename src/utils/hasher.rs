use std::io::{Error, Read};

use sha1::Sha1;

pub struct ReadHasher<R> {
    hash: Sha1,
    inner: R,
}

impl<R: Read> ReadHasher<R> {
    pub fn new(r: R) -> ReadHasher<R> {
        let hash = Sha1::new();
        ReadHasher { hash, inner: r }
    }

    pub fn digest_hex(self) -> String {
        self.hash.hexdigest()
    }
}

impl<R: Read> Read for ReadHasher<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let n = self.inner.read(buf)?;
        self.hash.update(&buf[..n]);
        Ok(n)
    }
}
