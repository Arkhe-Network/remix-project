// crates/arkhe-core/src/string_safe.rs
//! Wrapper seguro para strings com verificação formal Kani.

use core::slice;

/// String segura com proteção contra over-read (padrão Squidbleed).
#[derive(Debug, Clone)]
pub struct SafeString {
    inner: Vec<u8>,
}

impl SafeString {
    /// Cria uma nova string segura.
    pub fn new(bytes: &[u8]) -> Self {
        Self { inner: bytes.to_vec() }
    }

    /// Busca um caractere, garantindo que nunca ultrapasse o buffer.
    #[inline]
    pub fn find_char(&self, c: u8) -> Option<usize> {
        self.inner.iter().position(|&x| x == c)
    }

    /// Versão `unsafe` que deve ser provada correta via Kani.
    /// # Safety
    /// - `ptr` deve ser válido para `len` bytes.
    /// - O buffer pode conter bytes arbitrários (não necessariamente UTF-8).
    #[inline]
    pub unsafe fn find_char_raw(ptr: *const u8, len: usize, c: u8) -> Option<usize> {
        let slice = slice::from_raw_parts(ptr, len);
        slice.iter().position(|&x| x == c)
    }
}

impl Default for SafeString {
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}

// ============================================================
// Provas Kani
// ============================================================

#[cfg(kani)]
mod verification {
    use super::*;

    /// Provamos que find_char nunca retorna posição >= len.
    #[kani::proof]
    fn verify_find_char_bounds() {
        let bytes = kani::any::<[u8; 256]>();
        let s = SafeString { inner: bytes.to_vec() };
        let c = kani::any::<u8>();
        let result = s.find_char(c);

        match result {
            Some(pos) => {
                assert!(pos < s.inner.len());
                assert_eq!(s.inner[pos], c);
            }
            None => {
                assert!(!s.inner.contains(&c));
            }
        }
    }

    /// Provamos que find_char_raw nunca lê além do buffer.
    #[kani::proof]
    fn verify_find_char_raw_safety() {
        let bytes = kani::any::<[u8; 256]>();
        let len = kani::any::<usize>();
        kani::assume(len <= 256);

        let ptr = bytes.as_ptr();
        let c = kani::any::<u8>();

        unsafe {
            let result = SafeString::find_char_raw(ptr, len, c);
            match result {
                Some(pos) => {
                    assert!(pos < len);
                    assert_eq!(bytes[pos], c);
                }
                None => {
                    assert!(!bytes.iter().take(len).any(|&x| x == c));
                }
            }
        }
    }

    /// Provamos que o padrão Squidbleed NÃO acontece no SafeString.
    #[kani::proof]
    fn verify_no_squidbleed_pattern() {
        let bytes = kani::any::<[u8; 256]>();
        let s = SafeString { inner: bytes.to_vec() };
        let c = kani::any::<u8>();

        let result = s.find_char(c);
        match result {
            Some(pos) => assert!(pos < s.inner.len()),
            None => assert!(true),
        }
    }
}