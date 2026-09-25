use symphonia::core::formats::{FormatReader, SeekMode, SeekTo};
use symphonia::core::units::{Time, TimeBase};

#[derive(Debug, Clone, Copy)]
pub struct DurationResult {
    pub ms: u64,
    pub exact: bool,
}

fn ticks_to_ms(ticks: u64, time_base: TimeBase) -> u64 {
    let numer = time_base.numer.get() as u128;
    let denom = time_base.denom.get() as u128;
    ((ticks as u128 * numer * 1000) / denom) as u64
}

pub fn resolve_duration_ms(
    format: &mut Box<dyn FormatReader>,
    track_id: u32,
    file_size_bytes: Option<u64>,
) -> DurationResult {
    let track = format.tracks().iter().find(|t| t.id == track_id);
    if track.is_none() {
        return DurationResult {
            ms: 0,
            exact: false,
        };
    }

    let track = track.unwrap().clone();

    let sample_rate = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .and_then(|a| a.sample_rate);

    if let (Some(dur), Some(tb)) = (track.duration, track.time_base) {
        return DurationResult {
            ms: ticks_to_ms(dur.get(), tb),
            exact: true,
        };
    }

    if let (Some(dur), Some(sr)) = (track.duration, sample_rate) {
        return DurationResult {
            ms: dur.get().saturating_mul(1000) / sr as u64,
            exact: true,
        };
    }

    let mut last_ts_end: u64 = 0;
    let mut saw_any_packet = false;
    loop {
        match format.next_packet() {
            Ok(Some(packet)) => {
                if packet.track_id == track_id {
                    saw_any_packet = true;
                    last_ts_end = packet.pts.get() as u64 + packet.dur.get() as u64;
                }
            }
            Ok(None) | Err(_) => break,
        }
    }

    let _ = format.seek(
        SeekMode::Accurate,
        SeekTo::Time {
            time: Time::try_new(0, 0).expect("0,0 is always valid"),
            track_id: Some(track_id),
        },
    );

    if saw_any_packet {
        if let Some(tb) = track.time_base {
            return DurationResult {
                ms: ticks_to_ms(last_ts_end, tb),
                exact: true,
            };
        }
        if let Some(sr) = sample_rate {
            return DurationResult {
                ms: last_ts_end.saturating_mul(1000) / sr as u64,
                exact: true,
            };
        }
    }

    let bitrate = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .and_then(|a| a.bits_per_sample)
        .zip(sample_rate)
        .map(|(bits, sr)| bits as u64 * sr as u64 * 2);

    if let (Some(size), Some(bps)) = (file_size_bytes, bitrate) {
        if bps > 0 {
            return DurationResult {
                ms: size.saturating_mul(8000) / bps,
                exact: false,
            };
        }
    }

    DurationResult {
        ms: 0,
        exact: false,
    }
}
