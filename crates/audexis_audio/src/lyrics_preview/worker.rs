use super::*;

pub(super) fn run(
    path: String,
    commands: Receiver<Request>,
    ready: &Sender<Result<Status, String>>,
    equalizer_settings: Arc<Mutex<crate::equalizer::Settings>>,
) -> Result<(), String> {
    let (mut decoder, duration) = Decoder::open(&path)?;
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("No audio output device")?;
    let config = device
        .default_output_config()
        .map_err(|e| e.to_string())?
        .config();
    let rate = config.sample_rate;
    let channels = usize::from(config.channels);
    let output = Arc::new(Mutex::new(Output {
        duration_ms: duration,
        ..Output::default()
    }));
    let callback_output = output.clone();
    let error_output = output.clone();
    let initial_eq = *equalizer_settings
        .lock()
        .map_err(|_| "Equalizer unavailable")?;
    let mut equalizer = crate::equalizer::Equalizer::new(rate, channels, initial_eq);
    let mut last_start_ms = 0;
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                data.fill(0.0);
                if let Ok(mut state) = callback_output.try_lock() {
                    if !state.playing {
                        return;
                    }
                    let frames = (data.len() / channels).min(state.samples.len() / channels);
                    for sample in &mut data[..frames * channels] {
                        *sample = state.samples.pop_front().unwrap_or(0.0);
                    }
                    if state.start_ms != last_start_ms || state.frames == 0 {
                        equalizer.reset();
                        last_start_ms = state.start_ms;
                    }
                    if let Ok(settings) = equalizer_settings.try_lock() {
                        equalizer.update(*settings);
                    }
                    equalizer.process(&mut data[..frames * channels]);
                    state.frames += frames as u64;
                    if state.eof && state.samples.is_empty() {
                        state.playing = false;
                    }
                }
            },
            move |error| {
                if let Ok(mut state) = error_output.lock() {
                    state.error = Some(error.to_string());
                    state.playing = false;
                }
            },
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    let mut resampler = Async::<f32>::new_poly(
        rate as f64 / decoder.rate as f64,
        1.1,
        PolynomialDegree::Cubic,
        1024,
        decoder.channels,
        FixedAsync::Input,
    )
    .map_err(|e| e.to_string())?;
    let _ = ready.send(Ok(output
        .lock()
        .map_err(|_| "Preview unavailable")?
        .status(rate)));
    loop {
        let waiting = {
            let state = output.lock().map_err(|_| "Preview unavailable")?;
            !state.playing
                || state.eof
                || state.error.is_some()
                || state.samples.len() >= 8192 * channels
        };
        match commands.recv_timeout(Duration::from_millis(if waiting { 10 } else { 0 })) {
            Ok((action, ms, reply)) => {
                if matches!(action, Action::Close) {
                    let _ = reply.send(Ok(Status::default()));
                    return Ok(());
                }
                let result = (|| {
                    let mut state = output
                        .lock()
                        .map_err(|_| "Preview unavailable".to_string())?;
                    match action {
                        Action::Play => {
                            if state.eof && state.samples.is_empty() {
                                decoder.seek(0)?;
                                resampler.reset();
                                state.samples.clear();
                                state.frames = 0;
                                state.start_ms = 0;
                                state.eof = false;
                            }
                            state.playing = true;
                        }
                        Action::Pause => state.playing = false,
                        Action::Seek => {
                            let target = ms.min(duration);
                            decoder.seek(target)?;
                            resampler.reset();
                            state.samples.clear();
                            state.frames = 0;
                            state.start_ms = target;
                            state.eof = false;
                        }
                        _ => {}
                    }
                    Ok(state.status(rate))
                })();
                let _ = reply.send(result);
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
        }
        {
            let state = output.lock().map_err(|_| "Preview unavailable")?;
            if !state.playing
                || state.error.is_some()
                || state.samples.len() >= 8192 * channels
                || state.eof
            {
                continue;
            }
        }
        let required = resampler.input_frames_next() * decoder.channels;
        if decoder.pending.len() < required && !decoder.eof {
            if let Err(error) = decoder.fill() {
                let mut state = output.lock().map_err(|_| "Preview unavailable")?;
                state.error = Some(error);
                state.playing = false;
            }
            continue;
        }
        if decoder.pending.is_empty() && decoder.eof {
            output.lock().map_err(|_| "Preview unavailable")?.eof = true;
            continue;
        }
        let take = required.min(decoder.pending.len());
        let mut input: Vec<f32> = decoder.pending.drain(..take).collect();
        input.resize(required, 0.0);
        let mut samples = vec![0.0; resampler.output_frames_max() * decoder.channels];
        let capacity = samples.len() / decoder.channels;
        let (_, written) = resampler
            .process_into_buffer(
                &InterleavedSlice::new(&input, decoder.channels, required / decoder.channels)
                    .map_err(|e| e.to_string())?,
                &mut InterleavedSlice::new_mut(&mut samples, decoder.channels, capacity)
                    .map_err(|e| e.to_string())?,
                None,
            )
            .map_err(|e| e.to_string())?;
        let mut state = output.lock().map_err(|_| "Preview unavailable")?;
        for frame in samples[..written * decoder.channels].chunks_exact(decoder.channels) {
            for channel in 0..channels {
                state.samples.push_back(if channels == 1 {
                    frame.iter().sum::<f32>() / frame.len() as f32
                } else if decoder.channels == 1 {
                    frame[0]
                } else {
                    frame.get(channel).copied().unwrap_or(0.0)
                });
            }
        }
    }
}
// shout out chat gtt
