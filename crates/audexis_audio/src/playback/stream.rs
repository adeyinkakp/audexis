use super::*;

impl AudioPlayer {
    pub fn rebuild_active_stream(
        &mut self,
        player_arc_clone: Arc<Mutex<Self>>,
    ) -> Result<(), AudioPlayerError> {
        self.stream = None;

        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => return Err(AudioPlayerError::NoOutputDevice),
        };

        let device_config =
            device
                .default_output_config()
                .map_err(|error| AudioPlayerError::DeviceConfig {
                    message: error.to_string(),
                })?;
        let target_sample_rate = device_config.sample_rate();
        let channels = device_config.channels();

        let config = cpal::StreamConfig {
            channels,
            sample_rate: target_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let _ = self.cmd_tx.send(PlayerCmd::UpdateDeviceConfig {
            target_sample_rate,
            output_channels: channels as usize,
        });

        let consumer = Arc::clone(&self.consumer);
        let consumed_samples = Arc::clone(&self.consumed_samples);
        let flush_output = Arc::clone(&self.flush_output);
        let paused = Arc::clone(&self.paused);
        let position_ms = Arc::clone(&self.position_ms);
        let heard_us = Arc::clone(&self.heard_us);
        let output_sample_rate = target_sample_rate as u64;
        let output_channels = channels as u64;
        let mut progress = FrameProgress::default();
        let equalizer_settings = Arc::clone(&self.equalizer);
        let initial_eq = *equalizer_settings.lock().unwrap();
        let mut equalizer = super::super::equalizer::Equalizer::new(
            target_sample_rate,
            channels as usize,
            initial_eq,
        );

        let weak_player = Arc::downgrade(&player_arc_clone);
        let render = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            if let Ok(mut c) = consumer.try_lock() {
                if flush_output.load(Ordering::Acquire) {
                    let discarded = c.occupied_len();
                    c.clear();
                    consumed_samples.fetch_add(discarded as u64, Ordering::Release);
                    flush_output.store(false, Ordering::Release);
                    progress = FrameProgress::default();
                    equalizer.reset();
                }
                if paused.load(Ordering::Acquire) {
                    data.fill(0.0);
                    return;
                }
                let read = c.pop_slice(data);
                if let Ok(settings) = equalizer_settings.try_lock() {
                    equalizer.update(*settings);
                }

                equalizer.process(&mut data[..read]);
                if output_channels > 0 && output_sample_rate > 0 {
                    let frames = read as u64 / output_channels;
                    let (advance_ms, advance_us) = progress.advance(frames, output_sample_rate);
                    position_ms.fetch_add(advance_ms, Ordering::Relaxed);
                    heard_us.fetch_add(advance_us, Ordering::Relaxed);
                }
                consumed_samples.fetch_add(read as u64, Ordering::Release);
                if read < data.len() {
                    data[read..].fill(0.0);
                }
            } else {
                data.fill(0.0);
            }
        };
        let on_error = move |error| {
            tracing::error!(%error, "Audio output failed");
            let Some(player_inner) = weak_player.upgrade() else {
                return;
            };
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(150));
                if let Ok(mut locked) = player_inner.lock()
                    && let Err(error) = locked.rebuild_active_stream(player_inner.clone())
                {
                    tracing::error!("{error}");
                    locked.paused.store(true, Ordering::Release);
                    locked.events.state.send_modify(|state| {
                        state.paused = true;
                        state.error = Some(error.to_string());
                    });
                }
            });
        };
        let stream = match device_config.sample_format() {
            cpal::SampleFormat::F32 => build_stream::<f32>(&device, config, render, on_error),
            cpal::SampleFormat::F64 => build_stream::<f64>(&device, config, render, on_error),
            cpal::SampleFormat::I16 => build_stream::<i16>(&device, config, render, on_error),
            cpal::SampleFormat::I32 => build_stream::<i32>(&device, config, render, on_error),
            cpal::SampleFormat::U16 => build_stream::<u16>(&device, config, render, on_error),
            format => {
                return Err(AudioPlayerError::OutputStream {
                    message: format!("Unsupported output sample format: {format}"),
                });
            }
        }
        .map_err(|error| AudioPlayerError::OutputStream {
            message: error.to_string(),
        })?;

        stream
            .play()
            .map_err(|error| AudioPlayerError::StartStream {
                message: error.to_string(),
            })?;
        self.stream = Some(stream);
        Ok(())
    }
}

pub(super) fn build_stream<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut render: impl FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static,
    on_error: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<Stream, cpal::Error> {
    let mut scratch = vec![0.0f32; 1024 * usize::from(config.channels)];
    device.build_output_stream(
        config,
        move |data: &mut [T], info| {
            for chunk in data.chunks_mut(scratch.len()) {
                let samples = &mut scratch[..chunk.len()];
                render(samples, info);
                for (output, input) in chunk.iter_mut().zip(samples.iter()) {
                    *output = T::from_sample(*input);
                }
            }
        },
        on_error,
        None,
    )
}
