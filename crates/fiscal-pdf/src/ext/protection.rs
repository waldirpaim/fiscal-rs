//! Standard security handler, revision 2 (RC4, 40-bit key) — the "SetProtection" FPDF
//! script. Enabled with the `protection` cargo feature.
//!
//! **This scheme is obsolete** (MD5 + RC4 with a 40-bit key) and offers no real
//! confidentiality; it only asks well-behaved viewers to honour the permission flags.

use std::time::{SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};

use crate::document::Fpdf;

/// Padding string of the PDF standard security handler.
const PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08,
    0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

/// What a viewer should allow the user to do with a protected document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    /// Copy text and images.
    pub copy: bool,
    /// Print the document.
    pub print: bool,
    /// Modify the document.
    pub modify: bool,
    /// Add annotations and fill forms.
    pub annotate_forms: bool,
}

impl Permissions {
    /// Everything allowed.
    pub const ALL: Self = Self {
        copy: true,
        print: true,
        modify: true,
        annotate_forms: true,
    };
    /// Nothing allowed.
    pub const NONE: Self = Self {
        copy: false,
        print: false,
        modify: false,
        annotate_forms: false,
    };
}

/// Encryption state of a document.
#[derive(Debug, Clone)]
pub(crate) struct Encryption {
    key: Vec<u8>,
    o_value: Vec<u8>,
    u_value: Vec<u8>,
    p_value: i32,
    /// Object number of the encryption dictionary.
    pub(crate) obj_id: usize,
}

fn md5(data: &[u8]) -> Vec<u8> {
    Md5::digest(data).to_vec()
}

/// RC4 stream cipher.
fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut state: Vec<u8> = (0..=255).collect();
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + usize::from(state[i]) + usize::from(key[i % key.len()])) % 256;
        state.swap(i, j);
    }
    let (mut a, mut b) = (0usize, 0usize);
    data.iter()
        .map(|byte| {
            a = (a + 1) % 256;
            b = (b + usize::from(state[a])) % 256;
            state.swap(a, b);
            byte ^ state[(usize::from(state[a]) + usize::from(state[b])) % 256]
        })
        .collect()
}

fn padded(password: &[u8]) -> Vec<u8> {
    let mut v = password.to_vec();
    v.extend_from_slice(&PADDING);
    v.truncate(32);
    v
}

/// A unique-enough owner password when the caller gives none.
fn unique_id() -> Vec<u8> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let seed = format!("{nanos}-{}", std::process::id());
    md5(seed.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .into_bytes()
}

impl Encryption {
    fn new(user: &[u8], owner: &[u8], protection: u8) -> Self {
        let up = padded(user);
        let op = padded(owner);
        let o_value = rc4(&md5(&op)[..5], &up);
        let mut material = up;
        material.extend_from_slice(&o_value);
        material.extend_from_slice(&[protection, 255, 255, 255]);
        let key = md5(&material)[..5].to_vec();
        let u_value = rc4(&key, &PADDING);
        let p_value = -((i32::from(protection) ^ 255) + 1);
        Self {
            key,
            o_value,
            u_value,
            p_value,
            obj_id: 0,
        }
    }

    /// Key of the object numbered `n`.
    fn object_key(&self, n: usize) -> Vec<u8> {
        let mut material = self.key.clone();
        material.extend_from_slice(&[
            (n & 0xFF) as u8,
            (n >> 8 & 0xFF) as u8,
            (n >> 16 & 0xFF) as u8,
            0,
            0,
        ]);
        md5(&material)[..10].to_vec()
    }

    pub(crate) fn encrypt(&self, n: usize, data: &[u8]) -> Vec<u8> {
        rc4(&self.object_key(n), data)
    }

    pub(crate) fn dictionary_lines(&self) -> Vec<Vec<u8>> {
        let mut o = b"/O (".to_vec();
        o.extend(crate::encoding::escape(&self.o_value));
        o.push(b')');
        let mut u = b"/U (".to_vec();
        u.extend(crate::encoding::escape(&self.u_value));
        u.push(b')');
        vec![
            b"/Filter /Standard".to_vec(),
            b"/V 1".to_vec(),
            b"/R 2".to_vec(),
            o,
            u,
            format!("/P {}", self.p_value).into_bytes(),
        ]
    }
}

impl Fpdf {
    /// Protects the document with the permission flags `permissions`. Users must type
    /// `user_password` (may be empty) to open it; `owner_password` (random when empty) lifts
    /// the restrictions. Must be called before the document is closed.
    pub fn set_protection(
        &mut self,
        permissions: Permissions,
        user_password: &str,
        owner_password: &str,
    ) {
        let mut protection: u8 = 192;
        if permissions.print {
            protection += 4;
        }
        if permissions.modify {
            protection += 8;
        }
        if permissions.copy {
            protection += 16;
        }
        if permissions.annotate_forms {
            protection += 32;
        }
        let owner = if owner_password.is_empty() {
            unique_id()
        } else {
            owner_password.as_bytes().to_vec()
        };
        self.encryption = Some(Encryption::new(
            user_password.as_bytes(),
            &owner,
            protection,
        ));
    }

    /// Encrypts `data` for the object being written (identity when not protected).
    pub(crate) fn encrypt_for_object(&self, data: &[u8]) -> Vec<u8> {
        match &self.encryption {
            Some(enc) => enc.encrypt(self.n, data),
            None => data.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn md5_and_rc4_known_vectors() {
        assert_eq!(hex(&md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        // RFC 6229 style vector: key "Key", plaintext "Plaintext".
        assert_eq!(hex(&rc4(b"Key", b"Plaintext")), "bbf316e8d940af0ad3");
        // RC4 is symmetric.
        assert_eq!(
            rc4(b"Key", &rc4(b"Key", b"round trip")),
            b"round trip".to_vec()
        );
    }

    #[test]
    fn permission_flags_follow_the_pdf_specification() {
        let enc = Encryption::new(b"", b"owner", 192 + 4);
        assert_eq!(enc.p_value, -((196 ^ 255) + 1));
        assert_eq!(enc.key.len(), 5);
        assert_eq!(enc.o_value.len(), 32);
        assert_eq!(enc.u_value.len(), 32);
        assert_eq!(enc.object_key(7).len(), 10);
    }
}
