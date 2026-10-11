//! UTF-8 safe reading of hook input.
//!
//! Hook payloads arrive on stdin in arbitrary chunks, so a multi-byte character
//! can be split across two reads. Decoding each chunk on its own would turn the
//! two halves into replacement characters. Two entry points avoid that:
//!
//! - [`read_hook_input`] collects all bytes first and decodes once at the end.
//! - [`Utf8ChunkDecoder`] decodes incrementally and carries an incomplete
//!   trailing sequence over to the next chunk.

use std::io::{self, Read};

/// Size of the read buffer used by [`read_hook_input`].
const READ_CHUNK: usize = 8 * 1024;

/// Read all of `reader` and decode it as UTF-8, replacing invalid sequences.
pub fn read_hook_input<R: Read>(mut reader: R) -> io::Result<String> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Incremental UTF-8 decoder that keeps partial characters between chunks.
#[derive(Debug, Default)]
pub struct Utf8ChunkDecoder {
    pending: Vec<u8>,
}

impl Utf8ChunkDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one chunk and return the text that is complete so far.
    pub fn push(&mut self, chunk: &[u8]) -> String {
        todo!("stub")
    }

    /// Consume the decoder. A character that never completed becomes U+FFFD.
    pub fn finish(self) -> String {
        todo!("stub")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that returns at most `step` bytes per call.
    struct Trickle<'a> {
        data: &'a [u8],
        step: usize,
    }

    impl Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.step.min(buf.len()).min(self.data.len());
            buf[..n].copy_from_slice(&self.data[..n]);
            self.data = &self.data[n..];
            Ok(n)
        }
    }

    #[test]
    fn decoder_joins_three_byte_char_split_across_two_chunks() {
        // "—" is E2 80 94; split after the first byte.
        let bytes = "a—b".as_bytes();
        let mut dec = Utf8ChunkDecoder::new();
        let mut out = dec.push(&bytes[..2]);
        out.push_str(&dec.push(&bytes[2..]));
        out.push_str(&dec.finish());
        assert_eq!(out, "a—b");
    }

    #[test]
    fn decoder_joins_four_byte_char_split_into_single_bytes() {
        let text = "x😀y";
        let mut dec = Utf8ChunkDecoder::new();
        let mut out = String::new();
        for b in text.as_bytes() {
            out.push_str(&dec.push(std::slice::from_ref(b)));
        }
        out.push_str(&dec.finish());
        assert_eq!(out, text);
    }

    #[test]
    fn decoder_replaces_invalid_byte_and_continues() {
        let mut dec = Utf8ChunkDecoder::new();
        let mut out = dec.push(b"ok\xFFafter");
        out.push_str(&dec.finish());
        assert_eq!(out, "ok\u{FFFD}after");
    }

    #[test]
    fn decoder_turns_unfinished_tail_into_replacement_on_finish() {
        let mut dec = Utf8ChunkDecoder::new();
        let out = dec.push(b"abc\xE2\x80");
        assert_eq!(out, "abc");
        assert_eq!(dec.finish(), "\u{FFFD}");
    }

    #[test]
    fn read_hook_input_keeps_multibyte_json_across_tiny_reads() {
        let json = r#"{"tool_name":"smart_read","note":"Größe €😀 — ok"}"#;
        let reader = Trickle { data: json.as_bytes(), step: 1 };
        let out = read_hook_input(reader).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn read_hook_input_is_lossy_for_invalid_utf8_instead_of_failing() {
        let data: &[u8] = b"{\"a\":\"\xFF\"}";
        let out = read_hook_input(data).unwrap();
        assert!(out.contains('\u{FFFD}'));
        assert!(out.starts_with("{\"a\":\""));
    }
}
