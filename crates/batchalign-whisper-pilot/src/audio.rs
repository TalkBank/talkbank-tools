//! Audio decoding + resampling for the Whisper pilot.
//!
//! Originally vendored from `candle-examples/src/audio.rs` (Apache-2.0+MIT).
//! Re-ported here against the current dependency line:
//! - `pcm_decode` uses symphonia 0.6, whose API is a ground-up rework of the
//!   0.5 line (the `conv`/`sample`/`probe` top-level modules are gone; decoded
//!   audio is now a `GenericAudioBufferRef` rather than an `AudioBufferRef`
//!   enum). We extract channel 0 as mono f32, matching the original behavior.
//! - `resample` uses rubato 4.0, whose synchronous FFT resampler is now `Fft`
//!   (behind the `fft_resampler` feature) driven through the `audioadapter`
//!   buffer adapters, replacing the removed `FftFixedInOut` + slice API.
//!
//! `normalize_loudness` (upstream, `bs1770`-based) is intentionally omitted:
//! the pilot does not normalize loudness.

use candle_core::{Error, Result};

/// Decode any symphonia-supported audio file into f32 mono PCM (channel 0),
/// returning the samples and the source's sample rate in Hz.
pub fn pcm_decode<P: AsRef<std::path::Path>>(path: P) -> Result<(Vec<f32>, u32)> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::errors::Error as SymError;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    let src = std::fs::File::open(path).map_err(Error::wrap)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let hint = Hint::new();
    let fmt_opts: FormatOptions = Default::default();
    let meta_opts: MetadataOptions = Default::default();

    // symphonia 0.6: `probe()` returns the format reader directly (options are
    // passed by value), replacing 0.5's `format()` + `ProbeResult`.
    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, fmt_opts, meta_opts)
        .map_err(Error::wrap)?;

    // The default audio track, and its audio-specific codec parameters.
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| Error::Msg("no supported audio tracks".to_string()))?;
    let track_id = track.id;
    let audio_params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(|| Error::Msg("selected track has no audio codec parameters".to_string()))?;
    let sample_rate = audio_params.sample_rate.unwrap_or(0);

    let dec_opts: AudioDecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio_params, &dec_opts)
        .map_err(|_| Error::Msg("unsupported codec".to_string()))?;

    // The `track`/`audio_params` borrow of `format` ends here, before the loop
    // takes a mutable borrow via `next_packet`.
    let mut pcm_data: Vec<f32> = Vec::new();
    let mut interleaved: Vec<f32> = Vec::new();
    while let Some(packet) = format.next_packet().map_err(Error::wrap)? {
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Channel count = number of planes, floored at 1 so the
                // interleaving stride is always valid.
                let channels = decoded.num_planes().max(1);
                interleaved.clear();
                decoded.copy_to_vec_interleaved(&mut interleaved);
                // Keep channel 0 only (mono), matching the vendored behavior.
                pcm_data.extend(interleaved.iter().step_by(channels).copied());
            }
            // A single bad packet is skipped rather than aborting the decode.
            Err(SymError::DecodeError(_)) | Err(SymError::IoError(_)) => continue,
            Err(err) => return Err(Error::wrap(err)),
        }
    }
    Ok((pcm_data, sample_rate))
}

/// Resample mono PCM to a target sample rate using rubato 4.0's synchronous FFT
/// resampler, processing the whole clip in one `process_all_into_buffer` call.
pub fn resample(pcm_in: &[f32], sr_in: u32, sr_out: u32) -> Result<Vec<f32>> {
    use audioadapter_buffers::direct::InterleavedSlice;
    use rubato::{Fft, FixedSync, Resampler};

    // Mono: one channel, so frame count equals sample count.
    let channels = 1usize;
    let input_frames = pcm_in.len();
    if input_frames == 0 {
        return Ok(Vec::new());
    }

    let mut resampler = Fft::<f32>::new(
        sr_in as usize,
        sr_out as usize,
        1024, // chunk size; not load-bearing for FixedSync::Both offline batch use
        channels,
        FixedSync::Both,
    )
    .map_err(|e| Error::Msg(format!("rubato resampler construction failed: {e}")))?;

    // Output capacity: input frames scaled by the ratio, plus slack for the
    // resampler's internal delay/tail.
    let ratio = sr_out as f64 / sr_in as f64;
    let output_capacity = (input_frames as f64 * ratio) as usize + 1024;
    let mut output = vec![0f32; output_capacity * channels];

    let input_adapter = InterleavedSlice::new(pcm_in, channels, input_frames)
        .map_err(|e| Error::Msg(format!("rubato input adapter failed: {e}")))?;
    let mut output_adapter = InterleavedSlice::new_mut(&mut output, channels, output_capacity)
        .map_err(|e| Error::Msg(format!("rubato output adapter failed: {e}")))?;

    let (_frames_in, frames_out) = resampler
        .process_all_into_buffer(&input_adapter, &mut output_adapter, input_frames, None)
        .map_err(|e| Error::Msg(format!("rubato resample failed: {e}")))?;

    output.truncate(frames_out * channels);
    Ok(output)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use byteorder::{LittleEndian, WriteBytesExt};
    use std::io::Write;

    /// Synthesize a 1 kHz sine wave as mono f32 PCM at the given sample rate.
    fn sine(sample_rate: u32, seconds: f32) -> Vec<f32> {
        let n = (sample_rate as f32 * seconds) as usize;
        (0..n)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 1000.0 * t).sin()
            })
            .collect()
    }

    /// Write a minimal mono 16-bit PCM WAV to bytes, so pcm_decode can be
    /// exercised end-to-end without any external (or private) audio fixture.
    fn mono_pcm16_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut w = Vec::new();
        w.write_all(b"RIFF").unwrap();
        w.write_u32::<LittleEndian>(36 + data_len).unwrap();
        w.write_all(b"WAVE").unwrap();
        w.write_all(b"fmt ").unwrap();
        w.write_u32::<LittleEndian>(16).unwrap(); // PCM fmt chunk size
        w.write_u16::<LittleEndian>(1).unwrap(); // audio_format = PCM
        w.write_u16::<LittleEndian>(1).unwrap(); // channels = mono
        w.write_u32::<LittleEndian>(sample_rate).unwrap();
        w.write_u32::<LittleEndian>(sample_rate * 2).unwrap(); // byte_rate
        w.write_u16::<LittleEndian>(2).unwrap(); // block_align
        w.write_u16::<LittleEndian>(16).unwrap(); // bits_per_sample
        w.write_all(b"data").unwrap();
        w.write_u32::<LittleEndian>(data_len).unwrap();
        for &s in samples {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            w.write_i16::<LittleEndian>(v).unwrap();
        }
        w
    }

    #[test]
    fn resample_downsamples_to_target_length() {
        // 1 second at 48 kHz -> ~1 second at 16 kHz (a 3:1 downsample).
        let input = sine(48_000, 1.0);
        let out = resample(&input, 48_000, 16_000).unwrap();
        // Allow slack for the resampler's edge handling; the length must land
        // near the 1/3 ratio, not the input length.
        let expected = 16_000i64;
        assert!(
            (out.len() as i64 - expected).abs() < 2_000,
            "resampled length {} not near {}",
            out.len(),
            expected
        );
        assert!(
            out.iter().all(|v| v.is_finite()),
            "resample produced non-finite samples"
        );
    }

    #[test]
    fn resample_empty_input_is_empty() {
        let out = resample(&[], 44_100, 16_000).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn pcm_decode_reads_mono_wav_channel_and_rate() {
        let samples = sine(16_000, 0.25);
        let wav = mono_pcm16_wav(&samples, 16_000);
        let tmp = std::env::temp_dir().join("batchalign_whisper_pilot_decode_test.wav");
        std::fs::write(&tmp, &wav).unwrap();

        let (decoded, sample_rate) = pcm_decode(&tmp).unwrap();
        std::fs::remove_file(&tmp).ok();

        assert_eq!(sample_rate, 16_000);
        // Mono decode returns one sample per frame; allow a small tail delta
        // from codec framing.
        assert!(
            (decoded.len() as i64 - samples.len() as i64).abs() <= 8,
            "decoded {} samples, expected ~{}",
            decoded.len(),
            samples.len()
        );
        assert!(decoded.iter().all(|v| v.is_finite()));
    }
}
