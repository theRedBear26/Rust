
use fs4::fs_std::FileExt;
use std::{
    collections::{btree_map, BTreeMap},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    ops::{Bound, RangeBounds},
    path::PathBuf,
};

const KEY_VAL_HEADER_LEN: u32 = 4;
const MERGE_FILE_EXT: &str = "merge";

type KeyDir = BTreeMap<Vec<u8>, (u64, u32)>;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct MiniBitcask {
    log: Log,
    keydir: KeyDir,
}

impl Drop for MiniBitcask {
    fn drop(&mut self) {
        if let Err(error) = self.flush() {
            log::error!("Failed to flush file: {:?}", error);
        }
    }
}

impl MiniBitcask {
    pub fn new(path: PathBuf) -> Result<Self> {
        let mut log = Log::new(path)?;
        let keydir = log.load_index()?;

        Ok(Self { log, keydir })
    }

    pub fn set(&mut self, key: &[u8], value: Vec<u8>) -> Result<()> {
        let (value_pos, value_len) =
            self.log.write_entry(key, Some(&value))?;

        self.keydir
            .insert(key.to_vec(), (value_pos, value_len));

        Ok(())
    }

    pub fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        match self.keydir.get(key) {
            Some(&(value_pos, value_len)) => {
                Ok(Some(self.log.read_value(value_pos, value_len)?))
            }
            None => Ok(None),
        }
    }

    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.log.write_entry(key, None)?;
        self.keydir.remove(key);

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.log.file.sync_all()?;
        Ok(())
    }

    pub fn scan(
        &mut self,
        range: impl RangeBounds<Vec<u8>>,
    ) -> ScanIterator<'_> {
        ScanIterator {
            inner: self.keydir.range(range),
            log: &mut self.log,
        }
    }

    pub fn scan_prefix(&mut self, prefix: &[u8]) -> ScanIterator<'_> {
        let start = Bound::Included(prefix.to_vec());

        // Find the smallest lexicographic key greater than
        // every key starting with this prefix.
        let mut upper = prefix.to_vec();

        let end = loop {
            match upper.pop() {
                Some(byte) if byte < u8::MAX => {
                    upper.push(byte + 1);
                    break Bound::Excluded(upper);
                }
                Some(_) => continue,
                None => break Bound::Unbounded,
            }
        };

        self.scan((start, end))
    }

    pub fn merge(&mut self) -> Result<()> {
        self.flush()?;

        let mut merge_path = self.log.path.clone();
        merge_path.set_extension(MERGE_FILE_EXT);

        // Remove an old temporary merge file if one exists.
        if merge_path.exists() {
            std::fs::remove_file(&merge_path)?;
        }

        let mut new_log = Log::new(merge_path)?;
        let mut new_keydir = KeyDir::new();

        for (key, &(value_pos, value_len)) in &self.keydir {
            let value = self.log.read_value(value_pos, value_len)?;

            let (new_value_pos, new_value_len) =
                new_log.write_entry(key, Some(&value))?;

            new_keydir.insert(
                key.clone(),
                (new_value_pos, new_value_len),
            );
        }

        // Ensure the new log is written to disk before replacement.
        new_log.file.sync_all()?;

        // Replace the original log file.
        std::fs::rename(&new_log.path, &self.log.path)?;

        new_log.path = self.log.path.clone();

        self.log = new_log;
        self.keydir = new_keydir;

        Ok(())
    }
}

pub struct ScanIterator<'a> {
    inner: btree_map::Range<'a, Vec<u8>, (u64, u32)>,
    log: &'a mut Log,
}

impl<'a> ScanIterator<'a> {
    fn map(
        &mut self,
        item: (&Vec<u8>, &(u64, u32)),
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        let (key, &(value_pos, value_len)) = item;

        let value = self.log.read_value(value_pos, value_len)?;

        Ok((key.clone(), value))
    }
}

impl<'a> Iterator for ScanIterator<'a> {
    type Item = Result<(Vec<u8>, Vec<u8>)>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.inner.next()?;
        Some(self.map(item))
    }
}

impl<'a> DoubleEndedIterator for ScanIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let item = self.inner.next_back()?;
        Some(self.map(item))
    }
}

struct Log {
    path: PathBuf,
    file: std::fs::File,
}

impl Log {
    fn new(path: PathBuf) -> Result<Self> {
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir)?;
            }
        }

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        file.try_lock_exclusive()?;

        Ok(Self { path, file })
    }

    fn load_index(&mut self) -> Result<KeyDir> {
        let mut len_buf = [0u8; KEY_VAL_HEADER_LEN as usize];
        let mut keydir = KeyDir::new();

        let file_len = self.file.metadata()?.len();
        let mut reader = BufReader::new(&mut self.file);

        let mut pos = reader.seek(SeekFrom::Start(0))?;

        while pos < file_len {
            let record_start = pos;

            // Read key length.
            reader.read_exact(&mut len_buf)?;
            let key_len = u32::from_be_bytes(len_buf);

            // Read value length or tombstone marker.
            reader.read_exact(&mut len_buf)?;
            let raw_value_len = i32::from_be_bytes(len_buf);

            let value_len = match raw_value_len {
                -1 => None,
                len if len >= 0 => Some(len as u32),
                _ => return Err("Invalid value length or tombstone".into()),
            };

            // Read key.
            let mut key = vec![0u8; key_len as usize];
            reader.read_exact(&mut key)?;

            // Determine the position of the value.
            let value_pos =
                record_start + (KEY_VAL_HEADER_LEN * 2) as u64
                    + key_len as u64;

            match value_len {
                Some(len) => {
                    reader.seek(SeekFrom::Current(len as i64))?;

                    keydir.insert(key, (value_pos, len));

                    pos = value_pos + len as u64;
                }
                None => {
                    keydir.remove(&key);

                    pos = value_pos;
                }
            }

            if pos > file_len {
                return Err("Log record extends beyond end of file".into());
            }
        }

        Ok(keydir)
    }

    fn read_value(
        &mut self,
        value_pos: u64,
        value_len: u32,
    ) -> Result<Vec<u8>> {
        let mut value = vec![0u8; value_len as usize];

        self.file.seek(SeekFrom::Start(value_pos))?;
        self.file.read_exact(&mut value)?;

        Ok(value)
    }

    fn write_entry(
        &mut self,
        key: &[u8],
        value: Option<&[u8]>,
    ) -> Result<(u64, u32)> {
        let key_len = u32::try_from(key.len())?;

        let value_len = match value {
            Some(v) => u32::try_from(v.len())?,
            None => 0,
        };

        // The on-disk format uses i32 for value length.
        let value_len_or_tombstone = match value {
            Some(_) => i32::try_from(value_len)?,
            None => -1,
        };

        let offset = self.file.seek(SeekFrom::End(0))?;

        let value_pos = offset
            + (KEY_VAL_HEADER_LEN * 2) as u64
            + key_len as u64;

        {
            let mut writer =
                BufWriter::new(&mut self.file);

            writer.write_all(&key_len.to_be_bytes())?;

            writer.write_all(
                &value_len_or_tombstone.to_be_bytes(),
            )?;

            writer.write_all(key)?;

            if let Some(value) = value {
                writer.write_all(value)?;
            }

            writer.flush()?;
        }

        Ok((value_pos, value_len))
    }
}
