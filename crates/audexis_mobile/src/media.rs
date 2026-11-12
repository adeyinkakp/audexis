use super::*;

impl MobileCore {
    pub(super) fn commit_download(&mut self, request: &Value) -> Result<(), String> {
        let id = string(request, "id")?;
        let track = self.track(id)?.clone();
        let source_hash = string(request, "source_hash")?;
        if track.sha256 != source_hash {
            return Err("This track changed while downloading; try again".into());
        }
        let media: MediaUrl = serde_json::from_value(request["media"].clone())
            .map_err(|_| "Invalid download information")?;
        let path = PathBuf::from(string(request, "temporary")?);
        if !matches!(media.quality.as_str(), "original" | "aac_128" | "aac_256")
            || (media.quality == "original"
                && (media.sha256 != track.sha256 || media.size != track.size))
            || media.size <= 0
            || media.sha256.len() != 64
            || !media.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Invalid download information".into());
        }
        let mut file = fs::File::open(&path).map_err(|e| e.to_string())?;
        if file.metadata().map_err(|e| e.to_string())?.len() != media.size as u64 {
            return Err("Downloaded audio has the wrong size".into());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        if format!("{:x}", hash.finalize()) != media.sha256 {
            return Err("Downloaded audio failed verification".into());
        }
        let source_format = audio_format(&track);
        let extension = if media.quality == "original" {
            extension(&source_format)
        } else {
            "m4a"
        };
        let name = format!("{}.{extension}", media.sha256);
        let destination = self.root()?.join("audio").join(&name);
        if destination != path {
            let staged = destination.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
            let result = fs::copy(&path, &staged).and_then(|_| fs::rename(&staged, &destination));
            if result.is_err() {
                let _ = fs::remove_file(&staged);
            }
            result.map_err(|error| error.to_string())?;
        }
        self.state.downloads.insert(
            id.into(),
            Download {
                file: name,
                source_hash: track.sha256,
                sha256: media.sha256,
                size: media.size,
                quality: Some(media.quality),
            },
        );

        Ok(())
    }

    pub(super) fn media_plan(&mut self, request: &Value) -> Result<Value, String> {
        let track = self.track(string(request, "id")?)?;
        let automatic = match request.get("purpose").and_then(Value::as_str) {
            None | Some("streaming") => "aac_256",
            Some("download") => {
                if audio_format(track) == "spx" {
                    "aac_256"
                } else {
                    "original"
                }
            }
            _ => return Err("Unknown audio purpose".into()),
        };
        let quality = match request.get("quality") {
            None | Some(Value::Null) => automatic,
            Some(Value::String(quality)) if quality == "automatic" => automatic,
            Some(Value::String(quality))
                if matches!(quality.as_str(), "original" | "aac_128" | "aac_256") =>
            {
                quality.as_str()
            }
            _ => return Err("Unknown audio quality".into()),
        };
        Ok(
            json!({"path":format!("/api/tracks/{}/audio?quality={quality}", track.id),"quality":quality}),
        )
    }
}

pub(super) fn audio_format(track: &Track) -> String {
    Path::new(&track.file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or(&track.format)
        .to_ascii_lowercase()
}
fn extension(format: &str) -> &str {
    match format {
        "wave" => "wav",
        "aif" => "aiff",
        format
            if !format.is_empty()
                && format.len() <= 10
                && format.bytes().all(|byte| byte.is_ascii_alphanumeric()) =>
        {
            format
        }
        _ => "audio",
    }
}
