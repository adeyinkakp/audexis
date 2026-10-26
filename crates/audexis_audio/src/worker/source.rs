use super::state::WorkerState;
use crate::duration::resolve_duration_ms;
use symphonia::core::formats::{TrackType, probe::Hint};
use symphonia::core::io::MediaSourceStream;

pub fn open(
    path: &str,
    output_rate: u32,
    output_channels: usize,
) -> Result<(WorkerState, u64), String> {
    crate::decoder_guard::guard(|| open_inner(path, output_rate, output_channels))
}

fn open_inner(
    path: &str,
    output_rate: u32,
    output_channels: usize,
) -> Result<(WorkerState, u64), String> {
    let library = if path.starts_with("cloud-library://") {
        Some(crate::streaming::resolve_library(path)?)
    } else {
        None
    };
    let stream = match &library {
        Some(crate::streaming::LibrarySource::Stream(stream)) => Some(stream.clone()),
        _ if path.starts_with("cloud-stream://") => Some(crate::streaming::lookup(path)?),
        _ => None,
    };
    let path = match &library {
        Some(crate::streaming::LibrarySource::Local(path)) => {
            path.to_str().ok_or("Invalid local file path")?
        }
        _ => path,
    };
    let file: Box<dyn symphonia::core::io::MediaSource> = if let Some(stream) = &stream {
        Box::new(crate::streaming::HttpSource::new(stream.clone())?)
    } else {
        Box::new(std::fs::File::open(path).map_err(|e| e.to_string())?)
    };
    let mut hint = Hint::new();
    if let Some(stream) = &stream {
        hint.with_extension(&stream.extension);
    } else if let Some(ext) = std::path::Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
    {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            MediaSourceStream::new(file, Default::default()),
            Default::default(),
            Default::default(),
        )
        .map_err(|e| e.to_string())?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or("No audio track")?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or("No audio parameters")?;
    let rate = params
        .sample_rate
        .filter(|r| *r > 0)
        .ok_or("No sample rate")?;
    let channels = params
        .channels
        .as_ref()
        .map(|c| c.count())
        .filter(|c| *c > 0)
        .ok_or("No channels")?;
    let decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &Default::default())
        .map_err(|e| e.to_string())?;
    let track_id = track.id;
    let duration = if let Some(stream) = stream {
        if stream.duration_ms > 0 {
            stream.duration_ms
        } else {
            crate::duration::stream_duration_ms(&mut format, track_id)
        }
    } else {
        resolve_duration_ms(&mut format, track_id, None).ms
    };
    let mut state = WorkerState::new();
    state.track_id = track_id;
    state.source_sample_rate = rate;
    state.channels_uz = channels;
    state.target_sample_rate = output_rate;
    state.output_channels = output_channels;
    state.format = Some(format);
    state.decoder = Some(decoder);
    Ok((state, duration))
}
