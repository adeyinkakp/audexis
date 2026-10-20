use sha2::{Digest, Sha256};
use std::{
    io::{self, Read},
    path::Path,
};

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = crate::library_files::open(path)?;
    let before = file.metadata()?;
    if !before.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Not a regular file",
        ));
    }
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut bytes_read = 0u64;
    loop {
        let count = match file.read(&mut buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        bytes_read += count as u64;
    }
    let after = crate::library_files::metadata(path)?;
    if bytes_read != before.len()
        || after.len() != before.len()
        || after.modified()? != before.modified()?
    {
        return Err(io::Error::other("File changed while calculating SHA-256"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev() || before.ino() != after.ino() {
            return Err(io::Error::other(
                "File was replaced while calculating SHA-256",
            ));
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub(super) fn attempt_file_hash(path: &Path) -> Option<String> {
    match sha256_file(path) {
        Ok(hash) => Some(hash),
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "Could not calculate file SHA-256; will retry during library maintenance");
            None
        }
    }
}
