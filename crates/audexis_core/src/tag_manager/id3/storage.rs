use super::v2_common::create_header;
use crate::library_files;
use crate::tag_manager::tag_backend::{BackendError, TagError};
use crate::tag_manager::traits::Formats;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub(super) mod native;

#[derive(Clone, Copy)]
enum Container {
    Wave,
    Aiff,
}

struct Chunk {
    start: u64,
    size: u64,
    end: u64,
}

struct Layout {
    container: Container,
    end: u64,
    id3: Option<Chunk>,
    native: Vec<([u8; 4], Chunk)>,
}

impl Container {
    fn from_header(header: &[u8]) -> Option<Self> {
        match (header.get(..4)?, header.get(8..12)?) {
            (b"RIFF", b"WAVE") => Some(Self::Wave),
            (b"FORM", b"AIFF" | b"AIFC") => Some(Self::Aiff),
            _ => None,
        }
    }

    fn size(self, bytes: [u8; 4]) -> u64 {
        u64::from(match self {
            Self::Wave => u32::from_le_bytes(bytes),
            Self::Aiff => u32::from_be_bytes(bytes),
        })
    }

    fn encode_size(self, size: u64) -> io::Result<[u8; 4]> {
        let size = u32::try_from(size).map_err(|_| invalid("Container size exceeds 32 bits"))?;
        Ok(match self {
            Self::Wave => size.to_le_bytes(),
            Self::Aiff => size.to_be_bytes(),
        })
    }

    fn id3_name(self) -> &'static [u8; 4] {
        match self {
            Self::Wave => b"id3 ",
            Self::Aiff => b"ID3 ",
        }
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl Layout {
    fn read(file: &mut File, header: &[u8], container: Container) -> io::Result<Self> {
        let end = 8 + container.size(header[4..8].try_into().unwrap());
        if end < 12 || end > file.metadata()?.len() {
            return Err(invalid("Invalid audio container size"));
        }
        let mut offset = 12;
        let mut id3 = None;
        let mut native = Vec::new();
        while offset < end {
            if end - offset < 8 {
                return Err(invalid("Truncated audio chunk header"));
            }
            file.seek(SeekFrom::Start(offset))?;
            let mut chunk = [0; 8];
            file.read_exact(&mut chunk)?;
            let size = container.size(chunk[4..].try_into().unwrap());
            let data_end = offset + 8 + size;
            let next = data_end + (size % 2);
            if data_end > end {
                return Err(invalid("Truncated audio chunk"));
            }
            let name: [u8; 4] = chunk[..4].try_into().unwrap();
            let chunk = Chunk {
                start: offset,
                size,
                end: next.min(end),
            };
            if name.eq_ignore_ascii_case(b"ID3 ") {
                if id3.replace(chunk).is_some() {
                    return Err(invalid("Multiple embedded ID3 chunks"));
                }
            } else if matches!(
                (container, &name),
                (Container::Wave, b"LIST") | (Container::Aiff, b"NAME" | b"AUTH" | b"ANNO")
            ) {
                native.push((name, chunk));
            }
            offset = next;
        }
        Ok(Self {
            container,
            end,
            id3,
            native,
        })
    }

    fn open(file: &mut File) -> io::Result<Option<Self>> {
        let mut header = [0; 12];
        match file.read_exact(&mut header) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(error) => return Err(error),
        }
        Container::from_header(&header)
            .map(|container| Self::read(file, &header, container))
            .transpose()
    }
}

pub(in crate::tag_manager) fn is_container(header: &[u8]) -> bool {
    Container::from_header(header).is_some()
}

pub(in crate::tag_manager) fn container_format(
    file: &mut File,
    header: &[u8],
) -> Option<io::Result<Formats>> {
    let container = Container::from_header(header)?;
    Some((|| {
        let layout = Layout::read(file, header, container)?;
        let Some(chunk) = layout.id3 else {
            return Ok(Formats::Id3v23);
        };
        let mut signature = [0; 5];
        file.seek(SeekFrom::Start(chunk.start + 8))?;
        file.take(chunk.size).read_exact(&mut signature)?;
        match &signature {
            b"ID3\x02\0" => Ok(Formats::Id3v22),
            b"ID3\x03\0" => Ok(Formats::Id3v23),
            b"ID3\x04\0" => Ok(Formats::Id3v24),
            _ => Err(invalid("Unsupported embedded ID3 version")),
        }
    })())
}

pub(super) fn open(path: &Path, version: u8) -> io::Result<(Box<dyn Read>, u64)> {
    let mut file = library_files::open(path)?;
    let Some(layout) = Layout::open(&mut file)? else {
        file.seek(SeekFrom::Start(0))?;
        let size = file.metadata()?.len();
        return Ok((Box::new(file), size));
    };
    if let Some(chunk) = layout.id3 {
        file.seek(SeekFrom::Start(chunk.start + 8))?;
        Ok((Box::new(file.take(chunk.size)), chunk.size))
    } else {
        Ok((Box::new(Cursor::new(create_header(version, 0))), 10))
    }
}

pub(super) fn write(path: &Path, mut audio: impl Read, tag: &[u8]) -> io::Result<()> {
    let mut source = library_files::open(path)?;
    let layout = Layout::open(&mut source)?;
    let temporary = path.with_file_name(format!(".audexis-id3-{}.tmp", uuid::Uuid::new_v4()));
    let mut target = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        if let Some(layout) = layout {
            write_container(&mut source, &mut target, layout, tag)?;
        } else {
            target.write_all(tag)?;
            io::copy(&mut audio, &mut target)?;
        }
        target.set_permissions(source.metadata()?.permissions())?;
        target.sync_all()?;
        drop(target);
        drop(source);
        drop(audio);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn write_container(
    source: &mut File,
    target: &mut File,
    layout: Layout,
    tag: &[u8],
) -> io::Result<()> {
    let size = tag.len() as u64;
    let replaced = layout
        .id3
        .as_ref()
        .map_or(0, |chunk| chunk.end - chunk.start);
    let padding = (layout.end - replaced) % 2;
    let mut header = [0; 12];
    source.seek(SeekFrom::Start(0))?;
    source.read_exact(&mut header)?;
    header[4..8].copy_from_slice(
        &layout
            .container
            .encode_size(layout.end - replaced + padding + size + size % 2)?,
    );
    target.write_all(&header)?;

    if let Some(chunk) = layout.id3 {
        copy_bytes(source, target, chunk.start - 12)?;
        source.seek(SeekFrom::Start(chunk.end))?;
        copy_bytes(source, target, layout.end - chunk.end)?;
    } else {
        copy_bytes(source, target, layout.end - 12)?;
    }
    if padding != 0 {
        target.write_all(&[0])?;
    }

    target.write_all(layout.container.id3_name())?;
    target.write_all(&layout.container.encode_size(size)?)?;
    target.write_all(tag)?;
    if !size.is_multiple_of(2) {
        target.write_all(&[0])?;
    }
    io::copy(source, target)?;
    Ok(())
}

fn copy_bytes(source: &mut File, target: &mut File, size: u64) -> io::Result<()> {
    if io::copy(&mut source.take(size), target)? != size {
        return Err(invalid("Audio container changed during save"));
    }
    Ok(())
}

pub(super) fn tag_error(path: &Path, error: io::Error, writing: bool) -> BackendError {
    let details = TagError {
        path: path.to_string_lossy().into_owned(),
        public_message: if writing {
            "Could not write ID3 tag"
        } else {
            "Could not read ID3 tag"
        }
        .into(),
        internal_message: error.to_string(),
    };
    if writing {
        BackendError::WriteFailed(details)
    } else {
        BackendError::ReadFailed(details)
    }
}
