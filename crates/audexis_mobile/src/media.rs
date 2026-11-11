use super::*;

impl MobileCore {
    pub(super) fn commit_download(&mut self, request: &Value) -> Result<(), String> {
        let id = string(&request, "id")?;
        let track = self.track(id)?.clone();
        let source_hash = string(&request, "source_hash")?;
        if track.sha256 != source_hash {
            return Err("This track changed while downloading; try again".into());
        }
        let media: MediaUrl = serde_json::from_value(request["media"].clone())
            .map_err(|_| "Invalid download information")?;
        let path = PathBuf::from(string(&request, "temporary")?);
        if media.size <= 0
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
        let extension = if media.quality == "original" {
            extension(&audio_format(&track))
        } else {
            "m4a"
        };
        let name = format!("{}.{extension}", media.sha256);
        let destination = self.root()?.join("audio").join(&name);
        if destination != path {
            fs::copy(&path, &destination).map_err(|e| e.to_string())?;
        }
        self.state.downloads.insert(
            id.into(),
            Download {
                file: name,
                source_hash: track.sha256,
                sha256: media.sha256,
                size: media.size,
            },
        );

        Ok(())
    }

    pub(super) fn media_plan(&mut self, request: &Value) -> Result<Value, String> {
        let track = self.track(string(&request, "id")?)?;
        let automatic = if native_format(&audio_format(track)) {
            "original"
        } else {
            "aac_256"
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
        return Ok(
            json!({"path":format!("/api/tracks/{}/audio?quality={quality}", track.id),"quality":quality}),
        );
    }
}

pub(super) fn audio_format(track: &Track) -> String {
    Path::new(&track.file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or(&track.format)
        .to_ascii_lowercase()
}
fn native_format(format: &str) -> bool {
    matches!(
        format.to_ascii_lowercase().as_str(),
        "mp3" | "m4a" | "mp4" | "aac" | "wav" | "wave" | "aiff" | "aif" | "aifc" | "alac"
    )
}
fn extension(format: &str) -> &'static str {
    match format {
        "mp3" => "mp3",
        "aac" => "aac",
        "wav" | "wave" => "wav",
        "aiff" | "aif" => "aiff",
        "aifc" => "aifc",
        "flac" => "flac",
        "ogg" | "oga" => "ogg",
        "ogx" => "ogx",
        "ogm" => "ogm",
        "opus" => "opus",
        "mp1" => "mp1",
        "mp2" => "mp2",
        "spx" => "spx",
        "m4b" => "m4b",
        "mov" => "mov",
        "m4v" => "m4v",
        "ogv" => "ogv",
        _ => "m4a",
    }
}
