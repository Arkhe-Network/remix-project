// FILE: crates/safe-core-utils/src/persistence.rs
//! Persistencia append-only com fsync para decisoes arquiteturais.
//!
//! Design decisions:
//! - Cada decisao serializada e escrita como uma linha (delimitada por \\n)
//! - fsync apos cada write para durabilidade
//! - Recuperacao reconstrói o ArchitecturalAudit re-hashing cada decisao

use std::fs::{OpenOptions, File};
use std::io::{Write, BufRead, BufReader};
use std::path::Path;

/// Log append-only em arquivo com fsync.
#[derive(Debug)]
pub struct AppendOnlyLog {
    file: File,
    path: std::path::PathBuf,
}

impl AppendOnlyLog {
    /// Abre (ou cria) o arquivo de log em modo append.
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            file,
            path: path.as_ref().to_path_buf(),
        })
    }

    /// Escreve uma decisao serializada e forca fsync.
    pub fn append(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        // Encode bytes in base64 to avoid internal \n from breaking BufReader::lines
        use base64::{Engine as _, engine::general_purpose};
        let b64 = general_purpose::STANDARD.encode(bytes);
        self.file.write_all(b64.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.file.sync_all()?; // fsync para durabilidade
        Ok(())
    }

    /// Recupera todas as entradas do log como bytes.
    pub fn recover(&self) -> std::io::Result<Vec<Vec<u8>>> {
        use base64::{Engine as _, engine::general_purpose};
        let file = OpenOptions::new().read(true).open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if !line.is_empty() {
                if let Ok(decoded) = general_purpose::STANDARD.decode(&line) {
                    entries.push(decoded);
                } else {
                    // fallback to raw string decoding if not base64
                    entries.push(line.into_bytes());
                }
            }
        }
        Ok(entries)
    }
}
