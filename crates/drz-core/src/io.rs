use crate::document::Document;
use crate::error::CoreError;
use std::path::Path;

pub const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
const BINARY_SNIFF_LEN: usize = 8 * 1024;

impl Document {
    pub fn open(path: &Path) -> Result<Document, CoreError> {
        let meta = std::fs::metadata(path)?;
        if meta.len() > MAX_FILE_SIZE {
            return Err(CoreError::TooLarge(path.to_path_buf(), meta.len()));
        }
        let bytes = std::fs::read(path)?;
        let sniff = &bytes[..bytes.len().min(BINARY_SNIFF_LEN)];
        if sniff.contains(&0) {
            return Err(CoreError::BinaryFile(path.to_path_buf()));
        }
        let (text, guessed) = match std::str::from_utf8(&bytes) {
            Ok(s) => (s.to_string(), false),
            Err(_) => {
                let det = chardetng::EncodingDetector::new();
                let mut det = det;
                det.feed(&bytes, true);
                let enc = det.guess(None, true);
                let (cow, _, _) = enc.decode(&bytes);
                (cow.into_owned(), true)
            }
        };
        Ok(Document::from_file(text, path.to_path_buf(), guessed))
    }

    pub fn save(&mut self) -> Result<(), CoreError> {
        let path = self.path().ok_or(CoreError::NoPath)?.to_path_buf();
        std::fs::write(&path, self.to_string())?;
        self.mark_clean();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use std::io::Write;

    fn tmpfile(bytes: &[u8], name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("drzcore_test");
        std::fs::create_dir_all(&dir).ok();
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    #[test]
    fn open_utf8_roundtrip_save() {
        let p = tmpfile("fn main() {}\n".as_bytes(), "a.rs");
        let mut doc = Document::open(&p).unwrap();
        assert_eq!(doc.line(0), "fn main() {}");
        assert!(!doc.is_dirty());
        assert!(!doc.encoding_guessed());
        doc.replace_lines(0, 1, "fn main() { /*x*/ }");
        assert!(doc.is_dirty());
        doc.save().unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "fn main() { /*x*/ }\n");
    }

    #[test]
    fn open_binary_rejected() {
        let mut bytes = b"abc".to_vec();
        bytes.push(0);
        bytes.extend_from_slice(b"def");
        let p = tmpfile(&bytes, "bin.dat");
        assert!(matches!(Document::open(&p), Err(CoreError::BinaryFile(_))));
    }

    #[test]
    fn open_latin1_guessed() {
        // 0xE9 = é in latin-1, invalid UTF-8
        let p = tmpfile(&[0x63, 0x61, 0x66, 0xE9, 0x0A], "latin.txt");
        let doc = Document::open(&p).unwrap();
        assert!(doc.encoding_guessed());
        assert_eq!(doc.line(0), "café");
    }
}
