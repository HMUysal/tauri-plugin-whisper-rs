use log::{error, info};
use std::fs::File;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::conv::IntoSample;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::{Hint, ProbeResult};
use tauri::{AppHandle, Emitter, Runtime, Url};
use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};

use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Indexing, Resampler};

pub struct AudioProcessor<R: Runtime> {
    app: Option<AppHandle<R>>,
    file: Option<File>,
    probe: Option<ProbeResult>,
    accumulator: Vec<f32>,
    raw_input_buffer: Vec<f32>,
    resample_buffer: Vec<f32>,
    pub chunk_target: usize, // <--- Dışarıdan okunabilsin diye pub yapabilirsin veya getter yazabilirsin
    sample_rate: u32,
    resampler: Option<Fft<f32>>,
}

impl<R: Runtime> AudioProcessor<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        let default_seconds = 30.0;
        let chunk_target = (default_seconds * 16000.0) as usize;
        Self {
            app: Some(app),
            file: None,
            probe: None,
            accumulator: Vec::with_capacity(((default_seconds + 5.0) * 16000.0) as usize),
            raw_input_buffer: Vec::with_capacity(4096),
            resample_buffer: Vec::with_capacity(8192),
            chunk_target,
            sample_rate: 16000,
            resampler: None,
        }
    }

    pub fn set_chunk_target_seconds(&mut self, seconds: i32) {
        let target_samples = (seconds * 16000) as usize;

        self.chunk_target = target_samples;

        if self.accumulator.capacity() < target_samples {
            if target_samples > self.accumulator.len() {
                self.accumulator
                    .reserve(target_samples - self.accumulator.len());
            }
        }

        log::info!(
            "AudioProcessor: Chunk target updated to {} seconds ({} samples)",
            seconds,
            target_samples
        );
    }

    pub fn set_file(&mut self, path: &str) -> Result<(), String> {
        let mut options = OpenOptions::new();
        let read_options = options.read(true);

        let app = match self.app.as_mut() {
            Some(app) => app,
            None => {
                error!("AudioProcessor@set_file-app");
                return Err(format!("AudioProcessor@set_file-app"));
            }
        };

        let url = match Url::parse(path) {
            Ok(f) => f,
            Err(e) => {
                error!("AudioProcessor@set_file-url {:?}", e);
                match Url::from_file_path(path) {
                    Ok(u) => u,
                    Err(e) => {
                        error!("AudioProcessor@set_file-url {:?}", e);
                        return Err(format!("AudioProcessor@set_file-app"));
                    }
                }
            }
        };

        let file = match app.fs().open(FilePath::Url(url), read_options.clone()) {
            Ok(f) => f,
            Err(e) => {
                error!("AudioProcessor@set_file-file {:?}", e);
                return Err(format!("AudioProcessor@set_file-file {}", e));
            }
        };

        self.file = Some(file);
        Ok(())
    }

    pub fn set_file_info(&mut self) -> Result<(), String> {
        let file = match self.file.take() {
            Some(file) => file,
            None => {
                error!("AudioProcessor@set_file_info-file");
                return Err(format!("AudioProcessor@set_file_info-file"));
            }
        };

        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let probe = symphonia::default::get_probe();
        let probed = match probe.format(&Hint::new(), mss, &format_opts, &metadata_opts) {
            Ok(prboed) => prboed,
            Err(e) => {
                error!("AudioProcessor@set_file_info-probed {:?}", e);
                return Err(format!("AudioProcessor@set_file_info-probed {}", e));
            }
        };

        if let Some(track) = probed.format.tracks().first() {
            let s_rate = track.codec_params.sample_rate.unwrap_or(16000) as f32;

            if let Some(n_frames) = track.codec_params.n_frames {
                if let Some(app) = &self.app {
                    let _ = app.emit("transcription_total_progress", n_frames as f32 / s_rate);
                }
            } else {
                info!("AudioProcessor@set_file_info-no_frame");
            }
        }
        self.probe = Some(probed);
        Ok(())
    }

    pub fn start_decoding<F>(&mut self, mut callback: F) -> Result<(), String>
    where
        F: FnMut(Vec<f32>),
    {
        let mut format = match self.probe.take() {
            Some(probe) => probe.format,
            None => {
                error!("AudioProcessor@start_decoding-probe");
                return Err(format!("AudioProcessor@start_decoding-probe"));
            }
        };

        let track = match format.tracks().first() {
            Some(track) => track,
            None => {
                error!("AudioProcessor@start_decoding-track");
                return Err(format!("AudioProcessor@start_decoding-track"));
            }
        };

        self.sample_rate = track.codec_params.sample_rate.unwrap_or(16000);

        if self.sample_rate != 16000 {
            let resampler = match Fft::<f32>::new(
                self.sample_rate as usize,
                16000,
                1024,
                1,
                1,
                FixedSync::Both,
            ) {
                Ok(resampler) => resampler,
                Err(e) => {
                    error!("AudioProcessor@start_decoding-resampler {:?}", e);
                    return Err(format!("AudioProcessor@start_decoding-resampler {}", e));
                }
            };

            self.resampler = Some(resampler);
        } else {
            self.resampler = None;
        }

        let mut decoder =
            match symphonia::default::get_codecs().make(&track.codec_params, &Default::default()) {
                Ok(decoder) => decoder,
                Err(e) => {
                    error!("AudioProcessor@start_decoding-decoder {:?}", e);
                    return Err(format!("AudioProcessor@start_decoding-decoder {}", e));
                }
            };

        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(_) => break, // EOF
            };

            if let Ok(decoded) = decoder.decode(&packet) {
                self.process_audio_chunk(decoded, &mut callback);
            }
        }

        if let Some(ref mut resampler) = self.resampler {
            if !self.raw_input_buffer.is_empty() {
                let next_needed = resampler.input_frames_next();

                if self.raw_input_buffer.len() < next_needed {
                    self.raw_input_buffer.resize(next_needed, 0.0);
                }

                let input_adapter =
                    InterleavedSlice::new(&self.raw_input_buffer, 1, next_needed).unwrap();
                let estimated_output_frames = resampler.output_frames_next() + 512;

                if self.resample_buffer.len() < estimated_output_frames {
                    self.resample_buffer.resize(estimated_output_frames, 0.0);
                }

                let mut output_adapter = InterleavedSlice::new_mut(
                    &mut self.resample_buffer,
                    1,
                    estimated_output_frames,
                )
                .unwrap();

                let indexing = Indexing {
                    input_offset: 0,
                    output_offset: 0,
                    active_channels_mask: None,
                    partial_len: None,
                };

                if let Ok((_, frames_written)) = resampler.process_into_buffer(
                    &input_adapter,
                    &mut output_adapter,
                    Some(&indexing),
                ) {
                    self.accumulator
                        .extend_from_slice(&self.resample_buffer[0..frames_written]);
                }
                self.raw_input_buffer.clear();
            }
        }

        if !self.accumulator.is_empty() {
            info!(
                "AudioProcessor@start_decoding-chunk{}",
                self.accumulator.len()
            );
            let to_process = std::mem::take(&mut self.accumulator);
            callback(to_process);
        }

        Ok(())
    }

    fn process_audio_chunk<F>(&mut self, buffer: AudioBufferRef, mut callback: F)
    where
        F: FnMut(Vec<f32>),
    {
        match buffer {
            AudioBufferRef::U8(buf) => self.append_and_resample(&buf),
            AudioBufferRef::U16(buf) => self.append_and_resample(&buf),
            AudioBufferRef::U24(buf) => self.append_and_resample(&buf),
            AudioBufferRef::U32(buf) => self.append_and_resample(&buf),
            AudioBufferRef::S8(buf) => self.append_and_resample(&buf),
            AudioBufferRef::S16(buf) => self.append_and_resample(&buf),
            AudioBufferRef::S24(buf) => self.append_and_resample(&buf),
            AudioBufferRef::S32(buf) => self.append_and_resample(&buf),
            AudioBufferRef::F32(buf) => self.append_and_resample(&buf),
            AudioBufferRef::F64(buf) => self.append_and_resample(&buf),
        };

        while self.accumulator.len() >= self.chunk_target {
            self.split_and_dispatch(&mut callback);
        }
    }

    fn append_and_resample<S>(&mut self, buf: &symphonia::core::audio::AudioBuffer<S>)
    where
        S: symphonia::core::sample::Sample + IntoSample<f32>,
    {
        let frames = buf.frames();
        let channels = buf.spec().channels.count();

        let channels_data: Vec<&[S]> = (0..channels).map(|c| buf.chan(c)).collect();

        for i in 0..frames {
            let mut sample_sum = 0.0;
            for chan in &channels_data {
                sample_sum += chan[i].into_sample();
            }
            self.raw_input_buffer.push(sample_sum / channels as f32);
        }

        if let Some(ref mut resampler) = self.resampler {
            let mut input_frames_left = self.raw_input_buffer.len();
            let mut input_frames_next = resampler.input_frames_next();

            if input_frames_left >= input_frames_next {
                let input_adapter =
                    InterleavedSlice::new(&self.raw_input_buffer, 1, input_frames_left).unwrap();

                let estimated_output_frames =
                    (input_frames_left as f64 * 16000.0 / self.sample_rate as f64) as usize + 2048;

                if self.resample_buffer.len() < estimated_output_frames {
                    self.resample_buffer.resize(estimated_output_frames, 0.0);
                }

                let mut output_adapter = InterleavedSlice::new_mut(
                    &mut self.resample_buffer,
                    1,
                    estimated_output_frames,
                )
                .unwrap();

                let mut indexing = Indexing {
                    input_offset: 0,
                    output_offset: 0,
                    active_channels_mask: None,
                    partial_len: None,
                };

                while input_frames_left >= input_frames_next {
                    let (frames_read, frames_written) = resampler
                        .process_into_buffer(&input_adapter, &mut output_adapter, Some(&indexing))
                        .unwrap();

                    if frames_read == 0 {
                        break;
                    }

                    indexing.input_offset += frames_read;
                    indexing.output_offset += frames_written;
                    input_frames_left -= frames_read;
                    input_frames_next = resampler.input_frames_next();
                }

                self.accumulator
                    .extend_from_slice(&self.resample_buffer[0..indexing.output_offset]);

                self.raw_input_buffer.drain(0..indexing.input_offset);
            }
        } else {
            self.accumulator.append(&mut self.raw_input_buffer);
        }
    }

    fn split_and_dispatch<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Vec<f32>),
    {
        if self.accumulator.len() < self.chunk_target {
            return;
        }

        let search_window_back = (self.chunk_target / 6).max(1600);
        let search_start = self.chunk_target.saturating_sub(search_window_back);

        let window_size = 1600;

        let mut min_energy = f32::MAX;
        let mut cut_idx = self.chunk_target;

        let loop_end = self.chunk_target.saturating_sub(window_size);

        if search_start < loop_end {
            for i in (search_start..loop_end).step_by(window_size / 2) {
                let mut energy = 0.0;
                for j in 0..window_size {
                    energy += self.accumulator[i + j].abs();
                }

                if energy < min_energy {
                    min_energy = energy;
                    cut_idx = i + window_size / 2;
                }
            }
        }

        let to_process = self.accumulator[0..cut_idx].to_vec();
        self.accumulator.drain(0..cut_idx);

        callback(to_process);
    }
}
