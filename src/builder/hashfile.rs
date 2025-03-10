
use sha2::Digest;
use std::io;

pub struct HashedFile<W> {
    writer: W,
    hash: sha2::Sha256
}

impl<W> HashedFile<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            hash: sha2::Sha256::new()
        }
    }
}

impl<W> HashedFile<W> where W: io::Write {
    pub fn finish(mut self) -> io::Result<(W, Vec<u8>)> {
        io::Write::flush(&mut self)?;
        Ok((self.writer, self.hash.finalize().to_vec()))
    }
}

impl<W> io::Write for HashedFile<W> where W: io::Write {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hash.update(buf);
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
