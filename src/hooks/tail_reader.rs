//! Bounded backward line reader.
//!
//! Reads a file from its end in fixed-size chunks and yields lines newest
//! first. Memory use is bounded by `chunk_size + max_line_len` no matter how
//! large the file is. Lines longer than `max_line_len` are skipped and counted.
//! Splitting happens only on `\n` (ASCII), so a multi-byte UTF-8 character is
//! never split across two yielded lines.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub struct BackwardLines<R: Read + Seek> {
    reader: R,
    chunk_size: usize,
    max_line_len: usize,
    /// File offset of the first byte not yet read (everything before it is unread).
    pos: u64,
    /// Unemitted bytes in file order; always ends right before already-emitted data.
    buf: Vec<u8>,
    /// True while skipping the remainder of an oversized line.
    discarding: bool,
    /// True until the first piece has been examined (handles a trailing newline).
    at_eof: bool,
    finished: bool,
    skipped_oversize: usize,
    peak_buffer: usize,
}

impl<R: Read + Seek> BackwardLines<R> {
    pub fn new(mut reader: R, chunk_size: usize, max_line_len: usize) -> io::Result<Self> {
        let pos = reader.seek(SeekFrom::End(0))?;
        Ok(Self {
            reader,
            chunk_size: chunk_size.max(1),
            max_line_len: max_line_len.max(1),
            pos,
            buf: Vec::new(),
            discarding: false,
            at_eof: true,
            finished: false,
            skipped_oversize: 0,
            peak_buffer: 0,
        })
    }

    /// Next line (newest first), without its terminating `\n`.
    pub fn next_line(&mut self) -> io::Result<Option<Vec<u8>>> {
        loop {
            // A complete line exists in the buffer: split it off the end.
            if let Some(i) = self.buf.iter().rposition(|&b| b == b'\n') {
                let line = self.buf.split_off(i + 1);
                self.buf.truncate(i);
                if std::mem::replace(&mut self.at_eof, false) && line.is_empty() {
                    // The file ended with '\n': that is not an extra empty line.
                    continue;
                }
                if line.len() > self.max_line_len {
                    self.skipped_oversize += 1;
                    continue;
                }
                return Ok(Some(line));
            }

            if self.finished {
                return Ok(None);
            }

            if self.pos == 0 {
                // Start of file reached: whatever remains is the first line.
                self.finished = true;
                if self.discarding {
                    self.discarding = false;
                    self.skipped_oversize += 1;
                    return Ok(None);
                }
                let line = std::mem::take(&mut self.buf);
                if std::mem::replace(&mut self.at_eof, false) && line.is_empty() {
                    return Ok(None);
                }
                if line.len() > self.max_line_len {
                    self.skipped_oversize += 1;
                    return Ok(None);
                }
                return Ok(Some(line));
            }

            // No complete line yet: read the previous chunk from the file.
            let n = self.pos.min(self.chunk_size as u64) as usize;
            self.pos -= n as u64;
            self.reader.seek(SeekFrom::Start(self.pos))?;
            let mut chunk = vec![0u8; n];
            self.reader.read_exact(&mut chunk)?;

            if self.discarding {
                // Still inside an oversized line. Drop bytes until its start.
                if let Some(i) = chunk.iter().rposition(|&b| b == b'\n') {
                    self.buf = chunk[..i].to_vec();
                    self.discarding = false;
                    self.skipped_oversize += 1;
                }
                continue;
            }

            self.peak_buffer = self.peak_buffer.max(self.buf.len() + chunk.len());
            chunk.append(&mut self.buf);
            self.buf = chunk;

            if !self.buf.contains(&b'\n') && self.buf.len() > self.max_line_len {
                self.discarding = true;
                self.buf = Vec::new();
            }
        }
    }

    /// Number of lines dropped because they exceeded `max_line_len`.
    pub fn skipped_oversize(&self) -> usize {
        self.skipped_oversize
    }

    /// Largest number of bytes held at once (buffer plus the chunk being read).
    pub fn peak_buffer_bytes(&self) -> usize {
        self.peak_buffer
    }
}

/// Open `path` for backward line reading.
pub fn open_backward(
    path: &Path,
    chunk_size: usize,
    max_line_len: usize,
) -> io::Result<BackwardLines<File>> {
    BackwardLines::new(File::open(path)?, chunk_size, max_line_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn collect(data: &[u8], chunk: usize, max: usize) -> (Vec<String>, usize) {
        let mut it = BackwardLines::new(Cursor::new(data.to_vec()), chunk, max).unwrap();
        let mut out = Vec::new();
        while let Some(line) = it.next_line().unwrap() {
            out.push(String::from_utf8(line).unwrap());
        }
        (out, it.skipped_oversize())
    }

    #[test]
    fn yields_lines_newest_first_with_tiny_chunks() {
        let (lines, skipped) = collect(b"alpha\nbeta\ngamma\ndelta\nepsilon", 3, 1024);
        assert_eq!(lines, vec!["epsilon", "delta", "gamma", "beta", "alpha"]);
        assert_eq!(skipped, 0);
    }

    #[test]
    fn trailing_newline_does_not_produce_empty_first_line() {
        let (lines, _) = collect(b"one\ntwo\n", 4, 1024);
        assert_eq!(lines, vec!["two", "one"]);
    }

    #[test]
    fn empty_file_yields_nothing() {
        let (lines, _) = collect(b"", 8, 64);
        assert!(lines.is_empty());
    }

    #[test]
    fn multibyte_lines_stay_intact_across_chunk_boundaries() {
        let text = "erste Zeile — ä€\nzweite 😀 Zeile\ndritte\n";
        for chunk in 1..=9 {
            let (lines, _) = collect(text.as_bytes(), chunk, 1024);
            assert_eq!(lines, vec!["dritte", "zweite 😀 Zeile", "erste Zeile — ä€"], "chunk {chunk}");
        }
    }

    #[test]
    fn oversized_lines_are_skipped_and_counted() {
        let long = "x".repeat(500);
        let data = format!("short\n{long}\nafter\n{long}");
        let (lines, skipped) = collect(data.as_bytes(), 64, 100);
        assert_eq!(lines, vec!["after", "short"]);
        assert_eq!(skipped, 2);
    }

    #[test]
    fn memory_stays_bounded_on_large_file() {
        let line = "{\"event\":\"tool\",\"payload\":\"0123456789abcdef\"}\n";
        let data = line.repeat(100_000); // about 4 MB
        let chunk = 64 * 1024;
        let max = 4 * 1024;
        let mut it = BackwardLines::new(Cursor::new(data.into_bytes()), chunk, max).unwrap();
        let mut count = 0usize;
        while let Some(l) = it.next_line().unwrap() {
            assert!(l.starts_with(b"{\"event\""));
            count += 1;
        }
        assert_eq!(count, 100_000);
        assert!(
            it.peak_buffer_bytes() <= chunk + max,
            "peak {} exceeds bound {}",
            it.peak_buffer_bytes(),
            chunk + max
        );
    }
}
