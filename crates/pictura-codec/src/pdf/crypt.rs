//! The standard security handler (design D9): ISO 32000-1 §7.6.3 algorithms
//! 1–5 for revisions 2–4 and ISO 32000-2 algorithms 2.A/2.B/8–10 for
//! revision 6, both ways: the writer encrypts, and the reader (`Decryptor`,
//! algorithms 6 and 7 / 2.A) opens a file with its user or owner password.
//! RC4 and the CBC chaining are written here over the RustCrypto MD5 / AES /
//! SHA-2 primitives.

use std::cell::Cell;

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use md5::{Digest, Md5};
use sha2::{Sha256, Sha384, Sha512};

use super::object::{Dict, Obj};
use super::options::{Changes, Encryption, Printing, Security};

/// Algorithm 2's password padding string.
const PAD: [u8; 32] = [
    0x28, 0xbf, 0x4e, 0x5e, 0x4e, 0x75, 0x8a, 0x41, 0x64, 0x00, 0x4e, 0x56, 0xff, 0xfa, 0x01, 0x08,
    0x2e, 0x2e, 0x00, 0xb6, 0xd0, 0x68, 0x3e, 0x80, 0x2f, 0x0c, 0xa9, 0xfe, 0x64, 0x53, 0x69, 0x7a,
];

/// Deterministic bytes from a 32-byte seed (SHA-256 in counter mode), so a
/// fixed seed reproduces a file byte for byte (rule 6).
pub(crate) struct Entropy {
    seed: [u8; 32],
    counter: Cell<u64>,
}

impl Entropy {
    pub(crate) fn new(seed: [u8; 32]) -> Entropy {
        Entropy {
            seed,
            counter: Cell::new(0),
        }
    }

    pub(crate) fn bytes<const N: usize>(&self) -> [u8; N] {
        let mut out = [0u8; N];
        for chunk in out.chunks_mut(32) {
            let n = self.counter.get();
            self.counter.set(n + 1);
            let mut hash = Sha256::new();
            hash.update(self.seed);
            hash.update(n.to_le_bytes());
            chunk.copy_from_slice(&hash.finalize()[..chunk.len()]);
        }
        out
    }
}

/// RC4 (ARCFOUR) over `data` with `key`.
pub(crate) fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut s: [u8; 256] = std::array::from_fn(|i| i as u8);
    let mut j = 0u8;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }
    let (mut i, mut j) = (0u8, 0u8);
    data.iter()
        .map(|&b| {
            i = i.wrapping_add(1);
            j = j.wrapping_add(s[i as usize]);
            s.swap(i as usize, j as usize);
            b ^ s[s[i as usize].wrapping_add(s[j as usize]) as usize]
        })
        .collect()
}

fn aes_block(key: &[u8], block: &mut [u8]) {
    let block = GenericArray::from_mut_slice(block);
    match key.len() {
        16 => aes::Aes128::new(GenericArray::from_slice(key)).encrypt_block(block),
        _ => aes::Aes256::new(GenericArray::from_slice(key)).encrypt_block(block),
    }
}

/// AES-CBC over whole blocks of `data` (no padding added).
pub(crate) fn aes_cbc(key: &[u8], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut chain = *iv;
    for chunk in data.chunks(16) {
        let mut block = [0u8; 16];
        for (k, b) in block.iter_mut().enumerate() {
            *b = chunk.get(k).copied().unwrap_or(0) ^ chain[k];
        }
        aes_block(key, &mut block);
        out.extend_from_slice(&block);
        chain = block;
    }
    out
}

/// AES-CBC with PKCS#5 padding, the IV prefixed (PDF's AESV2/AESV3 streams).
fn aes_encrypt(key: &[u8], iv: [u8; 16], data: &[u8]) -> Vec<u8> {
    let pad = 16 - data.len() % 16;
    let mut padded = data.to_vec();
    padded.extend(std::iter::repeat_n(pad as u8, pad));
    let mut out = iv.to_vec();
    out.extend(aes_cbc(key, &iv, &padded));
    out
}

/// AES-CBC decryption over whole blocks of `data` (no padding removed).
fn aes_cbc_decrypt(key: &[u8], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut chain = *iv;
    for chunk in data.as_chunks::<16>().0 {
        let mut block = GenericArray::clone_from_slice(chunk);
        match key.len() {
            16 => aes::Aes128::new(GenericArray::from_slice(key)).decrypt_block(&mut block),
            _ => aes::Aes256::new(GenericArray::from_slice(key)).decrypt_block(&mut block),
        }
        out.extend(block.iter().zip(chain).map(|(b, c)| b ^ c));
        chain = *chunk;
    }
    out
}

/// The inverse of `aes_encrypt`: the IV is the first block; a malformed
/// padding is left in place.
fn aes_decrypt(key: &[u8], data: &[u8]) -> Vec<u8> {
    let Some((iv, body)) = data.split_first_chunk::<16>() else {
        return Vec::new();
    };
    let mut out = aes_cbc_decrypt(key, iv, body);
    let pad = out.last().copied().unwrap_or(0) as usize;
    if (1..=16).contains(&pad) && pad <= out.len() {
        out.truncate(out.len() - pad);
    }
    out
}

/// A password's bytes for revisions 2–4: Latin-1 when it fits, else UTF-8.
/// ponytail: PDFDocEncoding beyond Latin-1 is not mapped.
fn legacy_password(password: &str) -> Vec<u8> {
    if password.chars().all(|c| (c as u32) < 256) {
        password.chars().map(|c| c as u8).collect()
    } else {
        password.as_bytes().to_vec()
    }
}

fn padded(password: &[u8]) -> [u8; 32] {
    let mut out = PAD;
    let n = password.len().min(32);
    out[..n].copy_from_slice(&password[..n]);
    out[n..].copy_from_slice(&PAD[..32 - n]);
    out
}

/// The `/P` permission bits (1-based bits 3–6, 9–12; reserved bits set).
pub(crate) fn permissions(security: &Security) -> i32 {
    const PRINT: u32 = 1 << 2;
    const MODIFY: u32 = 1 << 3;
    const COPY: u32 = 1 << 4;
    const ANNOTATE: u32 = 1 << 5;
    const FILL: u32 = 1 << 8;
    const ACCESS: u32 = 1 << 9;
    const ASSEMBLE: u32 = 1 << 10;
    const PRINT_HIGH: u32 = 1 << 11;
    let mut p: u32 = 0xffff_f0c0;
    if security.permissions_password.is_empty() {
        p |= PRINT | MODIFY | COPY | ANNOTATE | FILL | ACCESS | ASSEMBLE | PRINT_HIGH;
        return p as i32;
    }
    p |= match security.printing {
        Printing::None => 0,
        Printing::LowResolution => PRINT,
        Printing::HighResolution => PRINT | PRINT_HIGH,
    };
    p |= match security.changes {
        Changes::None => 0,
        Changes::PageAssembly => ASSEMBLE,
        Changes::FormFill => FILL,
        Changes::Commenting => ANNOTATE | FILL,
        Changes::AnyExceptExtract => MODIFY | ANNOTATE | FILL | ASSEMBLE,
    };
    if security.copying {
        p |= COPY;
    }
    if security.accessibility {
        p |= ACCESS;
    }
    p as i32
}

/// Algorithm 2.B (revision 6 hash).
fn hash_2b(password: &[u8], salt: &[u8], udata: &[u8]) -> [u8; 32] {
    let mut k: Vec<u8> = Sha256::new()
        .chain_update(password)
        .chain_update(salt)
        .chain_update(udata)
        .finalize()
        .to_vec();
    let mut round = 0u32;
    loop {
        let mut k1 = Vec::with_capacity(64 * (password.len() + k.len() + udata.len()));
        for _ in 0..64 {
            k1.extend_from_slice(password);
            k1.extend_from_slice(&k);
            k1.extend_from_slice(udata);
        }
        let iv: [u8; 16] = k[16..32].try_into().expect("32+ byte hash");
        let e = aes_cbc(&k[..16], &iv, &k1);
        let selector = e[..16].iter().map(|&b| u32::from(b)).sum::<u32>() % 3;
        k = match selector {
            0 => Sha256::digest(&e).to_vec(),
            1 => Sha384::digest(&e).to_vec(),
            _ => Sha512::digest(&e).to_vec(),
        };
        round += 1;
        let last = u32::from(*e.last().expect("non-empty"));
        if round >= 64 && last + 32 <= round {
            break;
        }
    }
    k[..32].try_into().expect("32+ byte hash")
}

/// One file's encryption: the `/Encrypt` dictionary and the per-object cipher.
pub(crate) struct Encryptor {
    level: Encryption,
    key: Vec<u8>,
    pub(crate) dict: Dict,
    ivs: Entropy,
}

impl Encryptor {
    pub(crate) fn new(
        security: &Security,
        level: Encryption,
        id: &[u8; 16],
        entropy: &Entropy,
    ) -> Encryptor {
        let p = permissions(security);
        let encrypt_metadata = !(security.plaintext_metadata
            && matches!(level, Encryption::Aes128 | Encryption::Aes256));
        let ivs = Entropy::new(entropy.bytes::<32>());
        if level == Encryption::Aes256 {
            return Self::new_r6(security, p, encrypt_metadata, entropy, ivs);
        }
        let (v, r, n) = match level {
            Encryption::Rc4_40 => (1, 2, 5usize),
            Encryption::Rc4_128 => (2, 3, 16),
            _ => (4, 4, 16),
        };
        let user = padded(&legacy_password(&security.open_password));
        let owner_source = if security.permissions_password.is_empty() {
            &security.open_password
        } else {
            &security.permissions_password
        };
        // Algorithm 3: the owner entry.
        let owner_key = owner_rc4_key(&padded(&legacy_password(owner_source)), r, n);
        let mut o = rc4(&owner_key, &user);
        if r >= 3 {
            for i in 1..=19u8 {
                o = rc4(&xor_key(&owner_key, i), &o);
            }
        }
        let key = file_key(&user, &o, p, id, r, n, encrypt_metadata);
        let u = user_entry(&key, r, id);
        let mut dict = Dict::new()
            .with("Filter", Obj::name("Standard"))
            .with("V", Obj::Int(v))
            .with("R", Obj::Int(r))
            .with("Length", Obj::Int(n as i64 * 8))
            .with("O", Obj::Str(o))
            .with("U", Obj::Str(u))
            .with("P", Obj::Int(i64::from(p)));
        if r == 4 {
            dict = with_crypt_filter(dict, "AESV2", 16, encrypt_metadata);
        }
        Encryptor {
            level,
            key,
            dict,
            ivs,
        }
    }

    fn new_r6(
        security: &Security,
        p: i32,
        encrypt_metadata: bool,
        entropy: &Entropy,
        ivs: Entropy,
    ) -> Encryptor {
        // ponytail: ISO 32000-2 SASLpreps a revision 6 password before taking
        // its first 127 UTF-8 bytes; the raw bytes are used here, so only an
        // ASCII password is sure to match what a reader computes.
        let truncate = |s: &str| -> Vec<u8> { s.as_bytes()[..s.len().min(127)].to_vec() };
        let user = truncate(&security.open_password);
        let owner = if security.permissions_password.is_empty() {
            user.clone()
        } else {
            truncate(&security.permissions_password)
        };
        let key: [u8; 32] = entropy.bytes();
        let [uvs, uks, ovs, oks]: [[u8; 8]; 4] = std::array::from_fn(|_| entropy.bytes());
        let mut u = hash_2b(&user, &uvs, &[]).to_vec();
        u.extend_from_slice(&uvs);
        u.extend_from_slice(&uks);
        let ue = aes_cbc(&hash_2b(&user, &uks, &[]), &[0; 16], &key);
        let mut o = hash_2b(&owner, &ovs, &u).to_vec();
        o.extend_from_slice(&ovs);
        o.extend_from_slice(&oks);
        let oe = aes_cbc(&hash_2b(&owner, &oks, &u), &[0; 16], &key);
        let mut perms = [0u8; 16];
        perms[..4].copy_from_slice(&(p as u32).to_le_bytes());
        perms[4..8].fill(0xff);
        perms[8] = if encrypt_metadata { b'T' } else { b'F' };
        perms[9..12].copy_from_slice(b"adb");
        perms[12..].copy_from_slice(&entropy.bytes::<4>());
        aes_block(&key, &mut perms);
        let dict = Dict::new()
            .with("Filter", Obj::name("Standard"))
            .with("V", Obj::Int(5))
            .with("R", Obj::Int(6))
            .with("Length", Obj::Int(256))
            .with("O", Obj::Str(o))
            .with("U", Obj::Str(u))
            .with("OE", Obj::Str(oe))
            .with("UE", Obj::Str(ue))
            .with("Perms", Obj::Str(perms.to_vec()))
            .with("P", Obj::Int(i64::from(p)));
        Encryptor {
            level: Encryption::Aes256,
            key: key.to_vec(),
            dict: with_crypt_filter(dict, "AESV3", 32, encrypt_metadata),
            ivs,
        }
    }

    /// Encrypt one string or stream of object `num` (generation 0).
    pub(crate) fn encrypt(&self, num: u32, data: &[u8]) -> Vec<u8> {
        match self.level {
            Encryption::Aes256 => aes_encrypt(&self.key, self.ivs.bytes(), data),
            level => {
                let aes = level == Encryption::Aes128;
                let key = object_key(&self.key, num, 0, aes);
                if aes {
                    aes_encrypt(&key, self.ivs.bytes(), data)
                } else {
                    rc4(&key, data)
                }
            }
        }
    }
}

fn xor_key(key: &[u8], i: u8) -> Vec<u8> {
    key.iter().map(|b| b ^ i).collect()
}

/// Algorithm 3 steps a–d: the RC4 key the owner entry is encrypted with.
fn owner_rc4_key(owner: &[u8; 32], r: i64, n: usize) -> Vec<u8> {
    let mut hash = Md5::digest(owner).to_vec();
    if r >= 3 {
        for _ in 0..50 {
            hash = Md5::digest(&hash[..n]).to_vec();
        }
    }
    hash.truncate(n);
    hash
}

/// Algorithm 2: the file key from a padded user password.
fn file_key(
    user: &[u8; 32],
    o: &[u8],
    p: i32,
    id: &[u8],
    r: i64,
    n: usize,
    encrypt_metadata: bool,
) -> Vec<u8> {
    let mut md5 = Md5::new();
    md5.update(user);
    md5.update(o);
    md5.update((p as u32).to_le_bytes());
    md5.update(id);
    if r >= 4 && !encrypt_metadata {
        md5.update([0xff; 4]);
    }
    let mut hash = md5.finalize().to_vec();
    if r >= 3 {
        for _ in 0..50 {
            hash = Md5::digest(&hash[..n]).to_vec();
        }
    }
    hash.truncate(n);
    hash
}

/// Algorithms 4 and 5: the user entry for a file key.
fn user_entry(key: &[u8], r: i64, id: &[u8]) -> Vec<u8> {
    if r == 2 {
        return rc4(key, &PAD);
    }
    let mut x = rc4(
        key,
        &Md5::new().chain_update(PAD).chain_update(id).finalize(),
    );
    for i in 1..=19u8 {
        x = rc4(&xor_key(key, i), &x);
    }
    x.resize(32, 0);
    x
}

/// Algorithm 1: the key for one object's strings and streams (revisions 2–4).
fn object_key(key: &[u8], num: u32, generation: u16, aes: bool) -> Vec<u8> {
    let mut md5 = Md5::new();
    md5.update(key);
    md5.update(&num.to_le_bytes()[..3]);
    md5.update(generation.to_le_bytes());
    if aes {
        md5.update(b"sAlT");
    }
    md5.finalize()[..(key.len() + 5).min(16)].to_vec()
}

/// How one kind of data (strings or streams) is encrypted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Method {
    Identity,
    Rc4,
    Aes,
}

/// The standard security handler's `/Encrypt` entries a reader needs, and
/// the first element of the trailer's `/ID`.
pub(crate) struct Standard {
    pub r: i64,
    /// The file key length in bytes (revisions 2–4).
    pub length: usize,
    pub o: Vec<u8>,
    pub u: Vec<u8>,
    pub oe: Vec<u8>,
    pub ue: Vec<u8>,
    pub p: i32,
    pub encrypt_metadata: bool,
    pub id: Vec<u8>,
}

/// One file's decryption key, from its user or owner password.
pub(crate) struct Decryptor {
    key: Vec<u8>,
    /// Revisions 5 and 6 use the file key for every object.
    aes256: bool,
}

impl Decryptor {
    /// The file key `password` opens, or `None` when it is neither the user
    /// nor the owner password.
    pub(crate) fn open(s: &Standard, password: &str) -> Option<Decryptor> {
        if s.r >= 5 {
            return Self::open_aes256(s, password);
        }
        let n = if s.r == 2 { 5 } else { s.length.clamp(5, 16) };
        let as_user = |user: &[u8; 32]| {
            let key = file_key(user, &s.o, s.p, &s.id, s.r, n, s.encrypt_metadata);
            // Algorithm 6: revision 3+ compares only the first 16 bytes.
            let m = if s.r == 2 { 32 } else { 16 };
            let u = user_entry(&key, s.r, &s.id);
            (s.u.len() >= m && u[..m] == s.u[..m]).then_some(Decryptor { key, aes256: false })
        };
        let typed = padded(&legacy_password(password));
        if let Some(found) = as_user(&typed) {
            return Some(found);
        }
        // Algorithm 7: the owner password unwraps the padded user password.
        let owner_key = owner_rc4_key(&typed, s.r, n);
        let mut user = s.o.get(..32)?.to_vec();
        if s.r == 2 {
            user = rc4(&owner_key, &user);
        } else {
            for i in (0..=19u8).rev() {
                user = rc4(&xor_key(&owner_key, i), &user);
            }
        }
        as_user(&user.try_into().ok()?)
    }

    /// Algorithm 2.A (and Adobe's revision 5, a single SHA-256).
    fn open_aes256(s: &Standard, password: &str) -> Option<Decryptor> {
        // ponytail: as in `new_r6`, the password is not SASLprepped.
        let password = &password.as_bytes()[..password.len().min(127)];
        let hash = |salt: &[u8], udata: &[u8]| -> [u8; 32] {
            if s.r == 5 {
                Sha256::new()
                    .chain_update(password)
                    .chain_update(salt)
                    .chain_update(udata)
                    .finalize()
                    .into()
            } else {
                hash_2b(password, salt, udata)
            }
        };
        let (u, o) = (s.u.get(..48)?, s.o.get(..48)?);
        let (wrap, wrapped) = if hash(&u[32..40], &[]) == u[..32] {
            (hash(&u[40..48], &[]), s.ue.get(..32)?)
        } else if hash(&o[32..40], u) == o[..32] {
            (hash(&o[40..48], u), s.oe.get(..32)?)
        } else {
            return None;
        };
        Some(Decryptor {
            key: aes_cbc_decrypt(&wrap, &[0; 16], wrapped),
            aes256: true,
        })
    }

    /// Decrypt one string or stream of object `num` `generation`.
    pub(crate) fn decrypt(
        &self,
        num: u32,
        generation: u16,
        method: Method,
        data: &[u8],
    ) -> Vec<u8> {
        match method {
            Method::Identity => data.to_vec(),
            Method::Rc4 | Method::Aes if self.aes256 => aes_decrypt(&self.key, data),
            Method::Rc4 => rc4(&object_key(&self.key, num, generation, false), data),
            Method::Aes => aes_decrypt(&object_key(&self.key, num, generation, true), data),
        }
    }
}

fn with_crypt_filter(dict: Dict, method: &str, length: i64, encrypt_metadata: bool) -> Dict {
    let filter = Dict::new()
        .with("Type", Obj::name("CryptFilter"))
        .with("CFM", Obj::name(method))
        .with("AuthEvent", Obj::name("DocOpen"))
        .with("Length", Obj::Int(length));
    dict.with("CF", Dict::new().with("StdCF", filter.into()).into())
        .with("StmF", Obj::name("StdCF"))
        .with("StrF", Obj::name("StdCF"))
        .with("EncryptMetadata", Obj::Bool(encrypt_metadata))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc4_matches_the_published_vector() {
        // The classic "Key" / "Plaintext" test vector.
        assert_eq!(
            rc4(b"Key", b"Plaintext"),
            [0xbb, 0xf3, 0x16, 0xe8, 0xd9, 0x40, 0xaf, 0x0a, 0xd3]
        );
    }

    #[test]
    fn aes_cbc_matches_the_nist_vector() {
        // NIST SP 800-38A F.2.1 CBC-AES128.Encrypt, first block.
        let key = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let iv: [u8; 16] = std::array::from_fn(|i| i as u8);
        let plain = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a,
        ];
        assert_eq!(
            aes_cbc(&key, &iv, &plain),
            [
                0x76, 0x49, 0xab, 0xac, 0x81, 0x19, 0xb2, 0x46, 0xce, 0xe9, 0x8e, 0x9b, 0x12, 0xe9,
                0x19, 0x7d
            ]
        );
    }

    #[test]
    fn permissions_set_reserved_bits_and_the_chosen_rights() {
        let mut security = Security::default();
        assert_eq!(permissions(&security), -4, "everything allowed");
        security.permissions_password = "owner".into();
        security.printing = Printing::None;
        security.copying = false;
        let p = permissions(&security) as u32;
        assert_eq!(p & (1 << 2), 0, "no printing");
        assert_eq!(p & (1 << 4), 0, "no copying");
        assert_ne!(p & (1 << 9), 0, "accessibility kept");
        assert_eq!(p & 0xffff_f0c0, 0xffff_f0c0, "reserved bits set");
    }

    /// The reader's view of an `Encryptor`'s dictionary.
    fn standard(dict: &Dict, id: &[u8]) -> Standard {
        let bytes = |key: &str| match dict.get(key) {
            Some(Obj::Str(s)) => s.clone(),
            _ => Vec::new(),
        };
        let int = |key: &str| match dict.get(key) {
            Some(Obj::Int(i)) => *i,
            _ => 0,
        };
        Standard {
            r: int("R"),
            length: int("Length") as usize / 8,
            o: bytes("O"),
            u: bytes("U"),
            oe: bytes("OE"),
            ue: bytes("UE"),
            p: int("P") as i32,
            encrypt_metadata: !matches!(dict.get("EncryptMetadata"), Some(Obj::Bool(false))),
            id: id.to_vec(),
        }
    }

    #[test]
    fn the_user_and_owner_passwords_decrypt_what_was_encrypted() {
        let id = [9u8; 16];
        for (open, owner) in [("kooka", "owner"), ("kooka", ""), ("", "owner")] {
            let security = Security {
                open_password: open.into(),
                permissions_password: owner.into(),
                ..Security::default()
            };
            for level in [
                Encryption::Rc4_40,
                Encryption::Rc4_128,
                Encryption::Aes128,
                Encryption::Aes256,
            ] {
                let encryptor = Encryptor::new(&security, level, &id, &Entropy::new([3; 32]));
                let method = match level {
                    Encryption::Rc4_40 | Encryption::Rc4_128 => Method::Rc4,
                    _ => Method::Aes,
                };
                let sealed = encryptor.encrypt(12, b"layered pixels");
                let s = standard(&encryptor.dict, &id);
                for password in [open, owner].into_iter().filter(|p| !p.is_empty()) {
                    let opened = Decryptor::open(&s, password)
                        .unwrap_or_else(|| panic!("{level:?} opens with {password:?}"));
                    assert_eq!(opened.decrypt(12, 0, method, &sealed), b"layered pixels");
                }
                assert_eq!(
                    Decryptor::open(&s, "").is_some(),
                    open.is_empty(),
                    "{level:?}: the empty password opens only a file without an open password"
                );
                assert!(Decryptor::open(&s, "wrong").is_none(), "{level:?}");
            }
        }
    }

    #[test]
    fn a_fixed_seed_reproduces_the_dictionary() {
        let security = Security {
            open_password: "kooka".into(),
            ..Security::default()
        };
        let id = [7u8; 16];
        for level in [
            Encryption::Rc4_40,
            Encryption::Rc4_128,
            Encryption::Aes128,
            Encryption::Aes256,
        ] {
            let a = Encryptor::new(&security, level, &id, &Entropy::new([1; 32]));
            let b = Encryptor::new(&security, level, &id, &Entropy::new([1; 32]));
            assert_eq!(a.dict, b.dict);
            assert_eq!(a.encrypt(3, b"hello"), b.encrypt(3, b"hello"));
        }
    }
}
