use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use bytes::Bytes;
use futures::stream;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zip::read::ZipFile;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{ManifestEntry, invalid_archive};
use aphel_kernel::errors::AppError;
use aphel_sdk::platform::application::shared::ByteStream;

pub(crate) type DataRow = Map<String, Value>;

const MAX_ENTRY_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_IN_MEMORY_ENTRY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 500_000;
const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const MAX_LINE_BYTES: u64 = 64 * 1024 * 1024;
const STREAM_CHUNK_BYTES: usize = 1024 * 1024;

fn zip_error(error: impl std::fmt::Display) -> AppError {
    AppError::Internal(format!(
        "cannot build the organization export archive: {error}"
    ))
}

fn hex_digest(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) struct TempFile {
    path: PathBuf,
}

impl TempFile {
    pub(crate) fn new(prefix: &str, extension: &str) -> Self {
        Self {
            path: std::env::temp_dir().join(format!("{prefix}-{}.{extension}", Uuid::now_v7())),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn create(&self) -> Result<File, AppError> {
        create_private_file(&self.path)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path)
            && error.kind() != io::ErrorKind::NotFound
        {
            tracing::warn!(path = %self.path.display(), error = %error, "failed to remove a temporary file");
        }
    }
}

struct Bounded<R> {
    inner: R,
    remaining: u64,
}

impl<R: Read> Read for Bounded<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut probe = [0u8; 1];
            return if self.inner.read(&mut probe)? == 0 {
                Ok(0)
            } else {
                Err(io::Error::other(
                    "the entry is larger than its declared size",
                ))
            };
        }
        let wanted = buf
            .len()
            .min(usize::try_from(self.remaining).unwrap_or(usize::MAX));
        let read = self.inner.read(&mut buf[..wanted])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

pub(crate) struct RowReader {
    _temp: TempFile,
    reader: BufReader<File>,
    path: String,
    number: u64,
    line: Vec<u8>,
}

impl RowReader {
    pub(crate) fn next_row(&mut self) -> Result<Option<DataRow>, AppError> {
        loop {
            self.line.clear();
            let read = (&mut self.reader)
                .take(MAX_LINE_BYTES + 1)
                .read_until(b'\n', &mut self.line)
                .map_err(|error| invalid_archive(format!("cannot read {}: {error}", self.path)))?;
            if read == 0 {
                return Ok(None);
            }
            self.number += 1;
            if self.line.len() as u64 > MAX_LINE_BYTES {
                return Err(invalid_archive(format!(
                    "{} line {} is too long",
                    self.path, self.number
                )));
            }
            if self.line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            return match serde_json::from_slice::<Value>(&self.line) {
                Ok(Value::Object(row)) => Ok(Some(row)),
                _ => Err(invalid_archive(format!(
                    "{} line {} is not a JSON object",
                    self.path, self.number
                ))),
            };
        }
    }
}

pub(crate) fn create_private_file(path: &Path) -> Result<File, AppError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(path)
        .map_err(|error| AppError::Internal(format!("cannot create {}: {error}", path.display())))
}

struct HashingWriter<W> {
    inner: W,
    hasher: Sha256,
    bytes: u64,
}

impl<W: Write> HashingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            bytes: 0,
        }
    }

    fn finish(self) -> (W, u64, String) {
        (self.inner, self.bytes, hex_digest(self.hasher))
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hasher.update(&buf[..written]);
        self.bytes += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub(crate) fn sha256_file(path: &Path) -> Result<(u64, String), AppError> {
    let mut file = File::open(path)
        .map_err(|error| AppError::Internal(format!("cannot read {}: {error}", path.display())))?;
    let mut sink = HashingWriter::new(io::sink());
    io::copy(&mut file, &mut sink)
        .map_err(|error| AppError::Internal(format!("cannot read {}: {error}", path.display())))?;
    let (_, bytes, sha256) = sink.finish();
    Ok((bytes, sha256))
}

pub(crate) fn file_stream(path: &Path) -> Result<ByteStream, AppError> {
    let file = File::open(path)
        .map_err(|error| AppError::Internal(format!("cannot read {}: {error}", path.display())))?;
    Ok(Box::pin(stream::try_unfold(
        Some(file),
        |file| async move {
            let Some(mut file) = file else {
                return Ok(None);
            };
            let (file, chunk) = tokio::task::spawn_blocking(move || {
                let mut buffer = vec![0; STREAM_CHUNK_BYTES];
                let read = file.read(&mut buffer)?;
                buffer.truncate(read);
                Ok::<_, io::Error>((file, buffer))
            })
            .await
            .map_err(|error| AppError::Internal(format!("file read task failed: {error}")))?
            .map_err(|error| {
                AppError::Internal(format!("cannot read a temporary file: {error}"))
            })?;
            if chunk.is_empty() {
                Ok(None)
            } else {
                Ok(Some((Bytes::from(chunk), Some(file))))
            }
        },
    )))
}

struct OpenEntry {
    path: String,
    kind: String,
    name: Option<String>,
    count: u64,
    bytes: u64,
    hasher: Sha256,
}

pub(crate) struct ArchiveWriter {
    writer: ZipWriter<BufWriter<File>>,
    entries: Vec<ManifestEntry>,
    current: Option<OpenEntry>,
}

impl ArchiveWriter {
    pub(crate) fn create(path: &Path) -> Result<Self, AppError> {
        let file = create_private_file(path)?;
        Ok(Self {
            writer: ZipWriter::new(BufWriter::new(file)),
            entries: Vec::new(),
            current: None,
        })
    }

    pub(crate) fn start(
        &mut self,
        path: &str,
        kind: &str,
        name: Option<&str>,
    ) -> Result<(), AppError> {
        if self.current.is_some() {
            return Err(zip_error("an archive entry is still open"));
        }
        let method = if kind == "blob" {
            CompressionMethod::Stored
        } else {
            CompressionMethod::Deflated
        };
        self.writer
            .start_file(
                path,
                SimpleFileOptions::default()
                    .compression_method(method)
                    .large_file(true)
                    .unix_permissions(0o600),
            )
            .map_err(zip_error)?;
        self.current = Some(OpenEntry {
            path: path.to_owned(),
            kind: kind.to_owned(),
            name: name.map(str::to_owned),
            count: 0,
            bytes: 0,
            hasher: Sha256::new(),
        });
        Ok(())
    }

    pub(crate) fn write(&mut self, chunk: &[u8]) -> Result<(), AppError> {
        let Some(entry) = self.current.as_mut() else {
            return Err(zip_error("no archive entry is open"));
        };
        self.writer.write_all(chunk).map_err(zip_error)?;
        entry.hasher.update(chunk);
        entry.bytes += chunk.len() as u64;
        Ok(())
    }

    pub(crate) fn push_line(&mut self, value: &Value) -> Result<(), AppError> {
        let mut line = serde_json::to_vec(value)
            .map_err(|error| AppError::Internal(format!("cannot encode a JSON line: {error}")))?;
        line.push(b'\n');
        self.write(&line)?;
        if let Some(entry) = self.current.as_mut() {
            entry.count += 1;
        }
        Ok(())
    }

    fn close(&mut self, count: Option<u64>) -> Result<u64, AppError> {
        let Some(entry) = self.current.take() else {
            return Err(zip_error("no archive entry is open"));
        };
        let count = count.unwrap_or(entry.count);
        self.entries.push(ManifestEntry {
            path: entry.path,
            kind: entry.kind,
            name: entry.name,
            count,
            bytes: entry.bytes,
            sha256: hex_digest(entry.hasher),
        });
        Ok(count)
    }

    pub(crate) fn current_sha256(&self) -> Option<String> {
        self.current
            .as_ref()
            .map(|entry| hex_digest(entry.hasher.clone()))
    }

    pub(crate) fn end(&mut self) -> Result<u64, AppError> {
        self.close(None)
    }

    pub(crate) fn end_blob(&mut self) -> Result<u64, AppError> {
        self.close(Some(1))?;
        Ok(self.entries.last().map_or(0, |entry| entry.bytes))
    }

    pub(crate) fn abort(&mut self) -> Result<(), AppError> {
        if self.current.take().is_some() {
            self.writer.abort_file().map_err(zip_error)?;
        }
        Ok(())
    }

    pub(crate) fn add(
        &mut self,
        path: &str,
        kind: &str,
        name: Option<&str>,
        count: u64,
        content: &[u8],
    ) -> Result<(), AppError> {
        self.start(path, kind, name)?;
        self.write(content)?;
        self.close(Some(count))?;
        Ok(())
    }

    pub(crate) fn entries(&self) -> &[ManifestEntry] {
        &self.entries
    }

    pub(crate) fn finish(mut self, manifest: &[u8]) -> Result<(), AppError> {
        self.writer
            .start_file(
                super::MANIFEST_FILE,
                SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .unix_permissions(0o600),
            )
            .map_err(zip_error)?;
        self.writer.write_all(manifest).map_err(zip_error)?;
        let mut file = self.writer.finish().map_err(zip_error)?;
        file.flush().map_err(zip_error)?;
        file.get_ref().sync_all().map_err(zip_error)
    }
}

pub(crate) struct ArchiveReader {
    archive: Mutex<ZipArchive<BufReader<File>>>,
    names: BTreeMap<String, (usize, u64)>,
}

impl ArchiveReader {
    pub(crate) fn open(path: &Path) -> Result<Self, AppError> {
        let file = File::open(path).map_err(|error| {
            AppError::Configuration(format!("cannot read {}: {error}", path.display()))
        })?;
        let mut archive = ZipArchive::new(BufReader::new(file))
            .map_err(|error| invalid_archive(format!("the file is not a valid zip: {error}")))?;
        if archive.len() > MAX_ARCHIVE_ENTRIES {
            return Err(invalid_archive(format!(
                "the zip has more than {MAX_ARCHIVE_ENTRIES} entries"
            )));
        }
        let mut names = BTreeMap::new();
        let mut total: u64 = 0;
        for index in 0..archive.len() {
            let entry = archive
                .by_index_raw(index)
                .map_err(|error| invalid_archive(format!("unreadable zip entry: {error}")))?;
            if entry.is_dir() {
                continue;
            }
            let Some(path) = entry.enclosed_name() else {
                return Err(invalid_archive("the zip has an unsafe entry path"));
            };
            let path = path.to_string_lossy().replace('\\', "/");
            if entry.size() > MAX_ENTRY_BYTES {
                return Err(invalid_archive(format!("{path} is too large")));
            }
            total = total.saturating_add(entry.size());
            if total > MAX_TOTAL_UNCOMPRESSED_BYTES {
                return Err(invalid_archive(
                    "the zip expands to more than the allowed total size",
                ));
            }
            names.insert(path, (index, entry.size()));
        }
        Ok(Self {
            archive: Mutex::new(archive),
            names,
        })
    }

    fn with_entry<T>(
        &self,
        path: &str,
        read: impl FnOnce(&mut dyn Read, u64) -> Result<T, AppError>,
    ) -> Result<Option<T>, AppError> {
        let Some((index, declared)) = self.names.get(path).copied() else {
            return Ok(None);
        };
        let mut archive = self
            .archive
            .lock()
            .map_err(|_| AppError::Internal("the archive reader is poisoned".to_owned()))?;
        let entry: ZipFile<'_, BufReader<File>> = archive
            .by_index(index)
            .map_err(|error| invalid_archive(format!("cannot read {path}: {error}")))?;
        let mut bounded = Bounded {
            inner: entry,
            remaining: declared,
        };
        read(&mut bounded, declared).map(Some)
    }

    pub(crate) fn contains(&self, path: &str) -> bool {
        self.names.contains_key(path)
    }

    pub(crate) fn get(&self, path: &str) -> Result<Option<Vec<u8>>, AppError> {
        self.with_entry(path, |entry, declared| {
            if declared > MAX_IN_MEMORY_ENTRY_BYTES {
                return Err(invalid_archive(format!("{path} is too large")));
            }
            let mut data = Vec::with_capacity(usize::try_from(declared).unwrap_or_default());
            entry
                .read_to_end(&mut data)
                .map_err(|error| invalid_archive(format!("cannot read {path}: {error}")))?;
            Ok(data)
        })
    }

    pub(crate) fn require(&self, path: &str) -> Result<Vec<u8>, AppError> {
        self.get(path)?
            .ok_or_else(|| invalid_archive(format!("{path} is missing from the archive")))
    }

    pub(crate) fn json(&self, path: &str) -> Result<Value, AppError> {
        serde_json::from_slice(&self.require(path)?)
            .map_err(|error| invalid_archive(format!("{path} is not valid JSON: {error}")))
    }

    pub(crate) fn rows(&self, path: &str) -> Result<Option<RowReader>, AppError> {
        let temp = TempFile::new("aphel-org-restore", "jsonl");
        let file = temp.create()?;
        let Some(file) = self.with_entry(path, |entry, _| {
            let mut writer = BufWriter::new(file);
            io::copy(entry, &mut writer)
                .map_err(|error| invalid_archive(format!("cannot read {path}: {error}")))?;
            let mut file = writer.into_inner().map_err(|error| {
                AppError::Internal(format!("cannot write a temporary file: {error}"))
            })?;
            file.seek(SeekFrom::Start(0)).map_err(|error| {
                AppError::Internal(format!("cannot read a temporary file: {error}"))
            })?;
            Ok(file)
        })?
        else {
            return Ok(None);
        };
        Ok(Some(RowReader {
            _temp: temp,
            reader: BufReader::new(file),
            path: path.to_owned(),
            number: 0,
            line: Vec::new(),
        }))
    }

    pub(crate) fn lines(&self, path: &str) -> Result<Vec<DataRow>, AppError> {
        let Some(mut reader) = self.rows(path)? else {
            return Ok(Vec::new());
        };
        let mut rows = Vec::new();
        while let Some(row) = reader.next_row()? {
            rows.push(row);
        }
        Ok(rows)
    }

    pub(crate) fn each_row(
        &self,
        path: &str,
        mut visit: impl FnMut(DataRow) -> Result<(), AppError>,
    ) -> Result<(), AppError> {
        let Some(mut reader) = self.rows(path)? else {
            return Ok(());
        };
        while let Some(row) = reader.next_row()? {
            visit(row)?;
        }
        Ok(())
    }

    pub(crate) fn extract(
        &self,
        path: &str,
        destination: &Path,
    ) -> Result<Option<(u64, String)>, AppError> {
        self.with_entry(path, |entry, _| {
            let file = create_private_file(destination)?;
            let mut writer = HashingWriter::new(BufWriter::new(file));
            io::copy(entry, &mut writer)
                .map_err(|error| invalid_archive(format!("cannot read {path}: {error}")))?;
            let (mut file, bytes, sha256) = writer.finish();
            file.flush().map_err(|error| {
                AppError::Internal(format!("cannot write {}: {error}", destination.display()))
            })?;
            Ok((bytes, sha256))
        })
    }

    pub(crate) fn verify(&self, entries: &[ManifestEntry]) -> Result<(), AppError> {
        for entry in entries {
            let (bytes, sha256) = self
                .with_entry(&entry.path, |content, _| {
                    let mut sink = HashingWriter::new(io::sink());
                    io::copy(content, &mut sink).map_err(|error| {
                        invalid_archive(format!("cannot read {}: {error}", entry.path))
                    })?;
                    let (_, bytes, sha256) = sink.finish();
                    Ok((bytes, sha256))
                })?
                .ok_or_else(|| {
                    invalid_archive(format!("{} is missing from the archive", entry.path))
                })?;
            if sha256 != entry.sha256 || bytes != entry.bytes {
                return Err(invalid_archive(format!(
                    "{} does not match the checksum of the manifest",
                    entry.path
                )));
            }
        }
        Ok(())
    }
}
