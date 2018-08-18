use std::io::{Error, Read};

pub trait ProgressObserver {
    fn progress(&mut self, delta: u64);
    fn complete(&mut self);

    fn observe_read<R: Read>(self, r: R) -> ReadProgress<Self, R>
    where
        Self: Sized,
    {
        ReadProgress {
            observer: self,
            inner: r,
        }
    }
}

pub struct ReadProgress<P, R> {
    observer: P,
    inner: R,
}

impl<P: ProgressObserver, R: Read> Read for ReadProgress<P, R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.observer.progress(n as u64);
            return Ok(n);
        } else {
            self.observer.complete();
            return Ok(0);
        }
    }
}
