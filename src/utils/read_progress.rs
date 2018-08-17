use std::io::{Error, Read};
use std::sync::mpsc;

pub struct ReadProgress<R> {
    inner: R,
    progress_sender: mpsc::Sender<usize>,
}

impl<R: Read> Read for ReadProgress<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            let _ = self.progress_sender.send(n);
            return Ok(n);
        } else {
            return Ok(0);
        }
    }
}

pub struct ReadProgressSignal {
    progress: u64,
    progress_receiver: mpsc::Receiver<usize>,
}

impl Iterator for ReadProgressSignal {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        if let Ok(progress) = self.progress_receiver.recv() {
            self.progress += progress as u64;
            Some(self.progress)
        } else {
            None
        }
    }
}

pub fn read_progress<R: Read>(r: R) -> (ReadProgress<R>, ReadProgressSignal) {
    let (progress_sender, progress_receiver) = mpsc::channel();

    let read_progress = ReadProgress {
        inner: r,
        progress_sender,
    };
    let read_progress_signal = ReadProgressSignal {
        progress: 0,
        progress_receiver,
    };

    (read_progress, read_progress_signal)
}
