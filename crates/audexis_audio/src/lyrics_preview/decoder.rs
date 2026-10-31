use super::*;

pub(super) struct Decoder {
    pub(super) format: Box<dyn FormatReader>,
    pub(super) decoder: Box<dyn AudioDecoder>,
    pub(super) track_id: u32,
    pub(super) rate: u32,
    pub(super) channels: usize,
    pub(super) time_base: Option<TimeBase>,
    pub(super) skip_until_ms: Option<u64>,
    pub(super) pending: Vec<f32>,
    pub(super) eof: bool,
}
impl Decoder {
    pub(super) fn open(path: &str) -> Result<(Self, u64), String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
        {
            hint.with_extension(ext);
        }
        let mut format = symphonia::default::get_probe()
            .probe(
                &hint,
                MediaSourceStream::new(Box::new(file), Default::default()),
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
        let time_base = track.time_base;
        let duration = crate::duration::resolve_duration_ms(&mut format, track_id, None).ms;
        Ok((
            Self {
                format,
                decoder,
                track_id,
                rate,
                channels,
                time_base,
                skip_until_ms: None,
                pending: Vec::new(),
                eof: false,
            },
            duration,
        ))
    }
    pub(super) fn seek(&mut self, ms: u64) -> Result<(), String> {
        self.format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::try_new((ms / 1000) as i64, ((ms % 1000) * 1_000_000) as u32)
                        .ok_or("Invalid position")?,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| e.to_string())?;
        self.decoder.reset();
        self.pending.clear();
        self.eof = false;
        self.skip_until_ms = Some(ms);
        Ok(())
    }
    pub(super) fn fill(&mut self) -> Result<(), String> {
        let Some(packet) = self.format.next_packet().map_err(|e| e.to_string())? else {
            self.eof = true;
            return Ok(());
        };
        if packet.track_id != self.track_id {
            return Ok(());
        }
        let decoded = self.decoder.decode(&packet).map_err(|e| e.to_string())?;
        let mut samples = vec![0.0; decoded.samples_interleaved()];
        decoded.copy_to_slice_interleaved(&mut samples);
        let skip = if let (Some(target), Some(tb)) = (self.skip_until_ms, self.time_base) {
            let packet_frame =
                (i128::from(packet.pts.get()) * i128::from(tb.numer.get()) * i128::from(self.rate))
                    / i128::from(tb.denom.get());
            let target_frame = i128::from(target) * i128::from(self.rate) / 1000;
            (target_frame - packet_frame)
                .max(0)
                .min((samples.len() / self.channels) as i128) as usize
        } else {
            0
        };
        if skip * self.channels < samples.len() {
            self.skip_until_ms = None;
        }
        self.pending
            .extend_from_slice(&samples[skip * self.channels..]);
        Ok(())
    }
}
