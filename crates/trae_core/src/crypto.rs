use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use aes::{
    cipher::{
        block_padding::NoPadding, generic_array::GenericArray, BlockDecrypt, BlockDecryptMut,
        BlockEncryptMut, KeyInit, KeyIvInit,
    },
    Aes256,
};
use cbc::{Decryptor, Encryptor};
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha512;

use crate::{
    error::{CoreError, CoreResult},
    types::PreparedDatabase,
};

pub const KEY_HEX: &str = "3605f6691095a993f03d5009c918352ef5be31ae31e8f000212b81ff058da773";
const KEY_BYTES: [u8; 32] = [
    0x36, 0x05, 0xf6, 0x69, 0x10, 0x95, 0xa9, 0x93, 0xf0, 0x3d, 0x50, 0x09, 0xc9, 0x18, 0x35, 0x2e,
    0xf5, 0xbe, 0x31, 0xae, 0x31, 0xe8, 0xf0, 0x00, 0x21, 0x2b, 0x81, 0xff, 0x05, 0x8d, 0xa7, 0x73,
];

const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";
pub const PAGE_SIZE: usize = 4096;
pub const RESERVE: usize = 80;
pub const IV_SIZE: usize = 16;
pub const HMAC_SIZE: usize = 64;

type HmacSha512 = Hmac<Sha512>;

#[derive(Debug, Clone, Copy)]
struct WalFrame {
    page_no: u32,
    page_start: usize,
}

pub fn is_plain_sqlite(path: &Path) -> CoreResult<bool> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 16];
    if file.read(&mut magic)? != 16 {
        return Ok(false);
    }
    Ok(&magic == SQLITE_MAGIC)
}

pub fn verify_key_for_db(db_path: &Path) -> CoreResult<()> {
    let mut page = [0u8; PAGE_SIZE];
    File::open(db_path)?.read_exact(&mut page)?;
    let c1 = &page[16..32];
    let iv = &page[PAGE_SIZE - RESERVE..PAGE_SIZE - RESERVE + IV_SIZE];
    let cipher =
        Aes256::new_from_slice(&KEY_BYTES).map_err(|e| CoreError::Message(e.to_string()))?;
    let mut block = GenericArray::clone_from_slice(c1);
    cipher.decrypt_block(&mut block);
    let p1: Vec<u8> = block.iter().zip(iv).map(|(a, b)| a ^ b).collect();
    let page_size = u16::from_be_bytes([p1[0], p1[1]]);
    let expected = page_size == PAGE_SIZE as u16
        && matches!(p1[2], 1 | 2)
        && matches!(p1[3], 1 | 2)
        && p1[4] == RESERVE as u8
        && p1[5] == 0x40
        && p1[6] == 0x20
        && p1[7] == 0x20;
    if expected {
        Ok(())
    } else {
        Err(CoreError::Message(format!(
            "SQLCipher key verification failed for {}. The database may use a different key.",
            db_path.display()
        )))
    }
}

pub fn decrypt_db(input_path: &Path, output_path: &Path) -> CoreResult<()> {
    verify_key_for_db(input_path)?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut input = File::open(input_path)?;
    let mut output = File::create(output_path)?;
    let pages = input.metadata()?.len() as usize / PAGE_SIZE;
    for page_no in 1..=pages {
        let mut page = [0u8; PAGE_SIZE];
        input.read_exact(&mut page)?;
        let plain = decrypt_page(&page, page_no as u64)?;
        output.write_all(&plain)?;
    }
    output.flush()?;
    Ok(())
}

pub fn decrypt_page(page_data: &[u8], page_no: u64) -> CoreResult<Vec<u8>> {
    let encrypted = if page_no == 1 {
        &page_data[16..PAGE_SIZE - RESERVE]
    } else {
        &page_data[..PAGE_SIZE - RESERVE]
    };
    let iv = &page_data[PAGE_SIZE - RESERVE..PAGE_SIZE - RESERVE + IV_SIZE];
    let mut buf = encrypted.to_vec();
    let plain = Decryptor::<Aes256>::new_from_slices(&KEY_BYTES, iv)
        .map_err(|e| CoreError::Message(e.to_string()))?
        .decrypt_padded_mut::<NoPadding>(&mut buf)
        .map_err(|e| CoreError::Message(e.to_string()))?
        .to_vec();
    let mut page = Vec::with_capacity(PAGE_SIZE);
    if page_no == 1 {
        page.extend_from_slice(SQLITE_MAGIC);
    }
    page.extend_from_slice(&plain);
    page.resize(PAGE_SIZE, 0);
    Ok(page)
}

pub fn hmac_key(db_path: &Path) -> CoreResult<[u8; 32]> {
    let mut salt = [0u8; 16];
    File::open(db_path)?.read_exact(&mut salt)?;
    let hmac_salt: Vec<u8> = salt.iter().map(|byte| byte ^ 0x3a).collect();
    let mut out = [0u8; 32];
    pbkdf2_hmac::<Sha512>(&KEY_BYTES, &hmac_salt, 2, &mut out);
    Ok(out)
}

pub fn calculate_page_hmac(
    hmac_key: &[u8],
    encrypted_page: &[u8],
    page_no: u64,
) -> CoreResult<Vec<u8>> {
    let encrypted_payload = if page_no == 1 {
        &encrypted_page[16..PAGE_SIZE - RESERVE]
    } else {
        &encrypted_page[..PAGE_SIZE - RESERVE]
    };
    let iv = &encrypted_page[PAGE_SIZE - RESERVE..PAGE_SIZE - RESERVE + IV_SIZE];
    let mut mac = <HmacSha512 as Mac>::new_from_slice(hmac_key)
        .map_err(|e| CoreError::Message(e.to_string()))?;
    mac.update(encrypted_payload);
    mac.update(iv);
    mac.update(&(page_no as u32).to_le_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

pub fn check_page_hmacs(db_path: &Path, pages: &[u64]) -> CoreResult<Vec<(u64, bool)>> {
    let hmac_key = hmac_key(db_path)?;
    let mut file = File::open(db_path)?;
    let mut results = Vec::with_capacity(pages.len());
    for page_no in pages {
        let offset = (*page_no - 1) * PAGE_SIZE as u64;
        file.seek(SeekFrom::Start(offset))?;
        let mut page = [0u8; PAGE_SIZE];
        file.read_exact(&mut page)?;
        let stored = &page[PAGE_SIZE - RESERVE + IV_SIZE..PAGE_SIZE];
        let expected = calculate_page_hmac(&hmac_key, &page, *page_no)?;
        results.push((*page_no, stored == expected.as_slice()));
    }
    Ok(results)
}

pub fn encrypt_page_from_plain(
    plain_page: &[u8],
    original_encrypted_page: Option<&[u8]>,
    page_no: u64,
    hmac_key: &[u8],
    salt: Option<&[u8]>,
) -> CoreResult<Vec<u8>> {
    let iv = if let Some(original) = original_encrypted_page {
        original[PAGE_SIZE - RESERVE..PAGE_SIZE - RESERVE + IV_SIZE].to_vec()
    } else {
        let mut generated = vec![0u8; IV_SIZE];
        rand::thread_rng().fill_bytes(&mut generated);
        generated
    };

    let payload = if page_no == 1 {
        &plain_page[16..PAGE_SIZE - RESERVE]
    } else {
        &plain_page[..PAGE_SIZE - RESERVE]
    };
    let mut buf = payload.to_vec();
    let encrypted_payload = Encryptor::<Aes256>::new_from_slices(&KEY_BYTES, &iv)
        .map_err(|e| CoreError::Message(e.to_string()))?
        .encrypt_padded_mut::<NoPadding>(&mut buf, payload.len())
        .map_err(|e| CoreError::Message(e.to_string()))?
        .to_vec();

    let mut encrypted_page = Vec::with_capacity(PAGE_SIZE);
    if page_no == 1 {
        if let Some(original) = original_encrypted_page {
            encrypted_page.extend_from_slice(&original[..16]);
        } else if let Some(salt) = salt {
            encrypted_page.extend_from_slice(salt);
        } else {
            return Err(CoreError::Message("missing page 1 salt".to_string()));
        }
    }
    encrypted_page.extend_from_slice(&encrypted_payload);
    encrypted_page.extend_from_slice(&iv);
    encrypted_page.extend(std::iter::repeat(0).take(HMAC_SIZE));
    let page_hmac = calculate_page_hmac(hmac_key, &encrypted_page, page_no)?;
    encrypted_page.truncate(PAGE_SIZE - HMAC_SIZE);
    encrypted_page.extend_from_slice(&page_hmac);
    Ok(encrypted_page)
}

pub fn changed_pages(before: &Path, after: &Path) -> CoreResult<Vec<u64>> {
    let before_size = fs::metadata(before)?.len();
    let after_size = fs::metadata(after)?.len();
    let pages = before_size.max(after_size).div_ceil(PAGE_SIZE as u64);
    let mut a = File::open(before)?;
    let mut b = File::open(after)?;
    let mut changed = Vec::new();
    for page_no in 1..=pages {
        let mut left = vec![0u8; PAGE_SIZE];
        let mut right = vec![0u8; PAGE_SIZE];
        let left_len = a.read(&mut left)?;
        let right_len = b.read(&mut right)?;
        if left_len != right_len || left != right {
            changed.push(page_no);
        }
    }
    Ok(changed)
}

pub fn apply_encrypted_page_patch(
    live_db: &Path,
    _before_plain: &Path,
    patched_plain: &Path,
    pages: &[u64],
) -> CoreResult<()> {
    let hmac_key = hmac_key(live_db)?;
    let salt = {
        let mut salt = [0u8; 16];
        File::open(live_db)?.read_exact(&mut salt)?;
        salt
    };
    let patched_size = fs::metadata(patched_plain)?.len();
    let mut live = OpenOptions::new().read(true).write(true).open(live_db)?;
    let mut patched = File::open(patched_plain)?;
    let live_size = live.metadata()?.len();
    for page_no in pages {
        let offset = (*page_no - 1) * PAGE_SIZE as u64;
        let original = if offset + PAGE_SIZE as u64 <= live_size {
            live.seek(SeekFrom::Start(offset))?;
            let mut encrypted = vec![0u8; PAGE_SIZE];
            live.read_exact(&mut encrypted)?;
            Some(encrypted)
        } else {
            None
        };
        patched.seek(SeekFrom::Start(offset))?;
        let mut plain = vec![0u8; PAGE_SIZE];
        patched.read_exact(&mut plain)?;
        let encrypted_page = encrypt_page_from_plain(
            &plain,
            original.as_deref(),
            *page_no,
            &hmac_key,
            Some(&salt),
        )?;
        live.seek(SeekFrom::Start(offset))?;
        live.write_all(&encrypted_page)?;
    }
    live.set_len(patched_size)?;
    live.flush()?;
    Ok(())
}

pub fn prepare_plain_database(
    database_path: &Path,
    workspace_dir: &Path,
) -> CoreResult<PreparedDatabase> {
    fs::create_dir_all(workspace_dir)?;
    let snapshot = workspace_dir.join("database.snapshot.db");
    fs::copy(database_path, &snapshot)?;
    checkpoint_wal_into_copy(
        &snapshot,
        &database_path.with_file_name(format!(
            "{}-wal",
            database_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("database.db")
        )),
    )?;
    let plain_path = workspace_dir.join("database.plain.db");
    if is_plain_sqlite(&snapshot)? {
        fs::copy(&snapshot, &plain_path)?;
    } else {
        decrypt_db(&snapshot, &plain_path)?;
    }
    Ok(PreparedDatabase {
        encrypted_source: database_path.to_path_buf(),
        plain_path,
        workspace_dir: workspace_dir.to_path_buf(),
    })
}

pub fn checkpoint_wal_into_copy(db_copy: &Path, wal_path: &Path) -> CoreResult<()> {
    if !wal_path.exists() || fs::metadata(wal_path)?.len() < 32 {
        return Ok(());
    }
    let mut wal = Vec::new();
    File::open(wal_path)?.read_to_end(&mut wal)?;
    let magic = u32::from_be_bytes([wal[0], wal[1], wal[2], wal[3]]);
    if magic != 0x377f0682 && magic != 0x377f0683 {
        return Ok(());
    }
    let mut page_size = u32::from_be_bytes([wal[8], wal[9], wal[10], wal[11]]) as usize;
    if page_size == 0 {
        page_size = PAGE_SIZE;
    }
    if page_size != PAGE_SIZE {
        return Err(CoreError::Message(format!(
            "unsupported WAL page size {page_size}; expected {PAGE_SIZE}"
        )));
    }
    let frame_size = 24 + page_size;
    let salt_1 = u32::from_be_bytes([wal[16], wal[17], wal[18], wal[19]]);
    let salt_2 = u32::from_be_bytes([wal[20], wal[21], wal[22], wal[23]]);
    let mut frames = Vec::new();
    let mut last_commit: Option<(usize, u32)> = None;
    let mut offset = 32usize;
    while offset + frame_size <= wal.len() {
        let page_no = u32::from_be_bytes([
            wal[offset],
            wal[offset + 1],
            wal[offset + 2],
            wal[offset + 3],
        ]);
        let db_size = u32::from_be_bytes([
            wal[offset + 4],
            wal[offset + 5],
            wal[offset + 6],
            wal[offset + 7],
        ]);
        let frame_salt_1 = u32::from_be_bytes([
            wal[offset + 8],
            wal[offset + 9],
            wal[offset + 10],
            wal[offset + 11],
        ]);
        let frame_salt_2 = u32::from_be_bytes([
            wal[offset + 12],
            wal[offset + 13],
            wal[offset + 14],
            wal[offset + 15],
        ]);
        if page_no == 0 || frame_salt_1 != salt_1 || frame_salt_2 != salt_2 {
            break;
        }
        let page_start = offset + 24;
        frames.push(WalFrame {
            page_no,
            page_start,
        });
        if db_size > 0 {
            last_commit = Some((frames.len(), db_size));
        }
        offset += frame_size;
    }

    let Some((last_commit_frame_count, commit_db_pages)) = last_commit else {
        return Ok(());
    };

    let mut db = OpenOptions::new().read(true).write(true).open(db_copy)?;
    for frame in frames.iter().take(last_commit_frame_count) {
        let page_no = frame.page_no as u64;
        if page_no > 0 {
            let page_offset = (page_no - 1) * page_size as u64;
            db.seek(SeekFrom::Start(page_offset))?;
            db.write_all(&wal[frame.page_start..frame.page_start + page_size])?;
        }
    }
    db.set_len(commit_db_pages as u64 * page_size as u64)?;
    db.flush()?;
    Ok(())
}

pub fn checkpoint_wal_into_live(live_db: &Path) -> CoreResult<()> {
    let wal_path = live_db.with_file_name(format!(
        "{}-wal",
        live_db
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("database.db")
    ));
    checkpoint_wal_into_copy(live_db, &wal_path)?;
    if wal_path.exists() {
        fs::remove_file(&wal_path)?;
    }
    let shm_path = live_db.with_file_name(format!(
        "{}-shm",
        live_db
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("database.db")
    ));
    if shm_path.exists() {
        fs::remove_file(shm_path)?;
    }
    Ok(())
}

pub fn copy_live_triplet(live_db: &Path, backup_dir: &Path) -> CoreResult<Vec<PathBuf>> {
    fs::create_dir_all(backup_dir)?;
    let mut copied = Vec::new();
    for suffix in ["", "-wal", "-shm"] {
        let source = live_db.with_file_name(format!(
            "{}{}",
            live_db
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("database.db"),
            suffix
        ));
        if source.exists() {
            let target = backup_dir.join(source.file_name().unwrap());
            fs::copy(&source, &target)?;
            copied.push(target);
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        fs::{self, File},
        io::{Read, Write},
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{checkpoint_wal_into_copy, PAGE_SIZE};

    const SALT_1: u32 = 0x1234_5678;
    const SALT_2: u32 = 0x90ab_cdef;

    #[test]
    fn wal_checkpoint_honors_commit_db_size_truncation() {
        let dir = test_dir("wal-truncate");
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join("database.db");
        let wal = dir.join("database.db-wal");
        write_pages(&db, &[b'A', b'B', b'C']);
        write_wal(
            &wal,
            &[(3, 3, filled_page(b'D')), (2, 2, filled_page(b'E'))],
        );

        checkpoint_wal_into_copy(&db, &wal).unwrap();

        assert_eq!(fs::metadata(&db).unwrap().len(), (PAGE_SIZE * 2) as u64);
        assert_eq!(read_page_byte(&db, 1), b'A');
        assert_eq!(read_page_byte(&db, 2), b'E');
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn wal_checkpoint_ignores_frames_after_last_commit() {
        let dir = test_dir("wal-uncommitted-tail");
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join("database.db");
        let wal = dir.join("database.db-wal");
        write_pages(&db, &[b'A', b'B']);
        write_wal(
            &wal,
            &[(2, 2, filled_page(b'C')), (2, 0, filled_page(b'D'))],
        );

        checkpoint_wal_into_copy(&db, &wal).unwrap();

        assert_eq!(fs::metadata(&db).unwrap().len(), (PAGE_SIZE * 2) as u64);
        assert_eq!(read_page_byte(&db, 2), b'C');
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn wal_checkpoint_ignores_wal_without_commit_frame() {
        let dir = test_dir("wal-no-commit");
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join("database.db");
        let wal = dir.join("database.db-wal");
        write_pages(&db, &[b'A']);
        write_wal(&wal, &[(1, 0, filled_page(b'Z'))]);

        checkpoint_wal_into_copy(&db, &wal).unwrap();

        assert_eq!(fs::metadata(&db).unwrap().len(), PAGE_SIZE as u64);
        assert_eq!(read_page_byte(&db, 1), b'A');
        let _ = fs::remove_dir_all(dir);
    }

    fn test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("lcb-{name}-{nanos}"))
    }

    fn write_pages(path: &Path, markers: &[u8]) {
        let mut file = File::create(path).unwrap();
        for marker in markers {
            file.write_all(&filled_page(*marker)).unwrap();
        }
    }

    fn write_wal(path: &Path, frames: &[(u32, u32, Vec<u8>)]) {
        let mut file = File::create(path).unwrap();
        file.write_all(&0x377f0682u32.to_be_bytes()).unwrap();
        file.write_all(&3007000u32.to_be_bytes()).unwrap();
        file.write_all(&(PAGE_SIZE as u32).to_be_bytes()).unwrap();
        file.write_all(&1u32.to_be_bytes()).unwrap();
        file.write_all(&SALT_1.to_be_bytes()).unwrap();
        file.write_all(&SALT_2.to_be_bytes()).unwrap();
        file.write_all(&0u32.to_be_bytes()).unwrap();
        file.write_all(&0u32.to_be_bytes()).unwrap();
        for (page_no, db_size, page) in frames {
            file.write_all(&page_no.to_be_bytes()).unwrap();
            file.write_all(&db_size.to_be_bytes()).unwrap();
            file.write_all(&SALT_1.to_be_bytes()).unwrap();
            file.write_all(&SALT_2.to_be_bytes()).unwrap();
            file.write_all(&0u32.to_be_bytes()).unwrap();
            file.write_all(&0u32.to_be_bytes()).unwrap();
            file.write_all(page).unwrap();
        }
    }

    fn filled_page(marker: u8) -> Vec<u8> {
        vec![marker; PAGE_SIZE]
    }

    fn read_page_byte(path: &Path, page_no: usize) -> u8 {
        let mut file = File::open(path).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        data[(page_no - 1) * PAGE_SIZE]
    }
}
