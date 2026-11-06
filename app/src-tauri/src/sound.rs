//! A short chime when a focus session or a break ends. Made in code, so there's no sound file to
//! ship, and played with Windows' own `PlaySound`, so it works with the window closed.

use std::sync::OnceLock;

use windows::core::PCWSTR;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

const SAMPLE_RATE: u32 = 22_050;
const NOTE_SEC: f32 = 0.55;
const VOLUME: f32 = 0.35;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chime {
    /// Two falling notes: put it down.
    FocusDone,
    /// Two rising notes: back to it.
    BreakDone,
}

impl Chime {
    fn notes(self) -> &'static [f32] {
        match self {
            Chime::FocusDone => &[784.0, 523.25], // G5, C5
            Chime::BreakDone => &[523.25, 784.0], // C5, G5
        }
    }
}

/// A 16-bit mono WAV of soft bell notes, each fading out.
fn wav(notes: &[f32]) -> Vec<u8> {
    let per_note = (SAMPLE_RATE as f32 * NOTE_SEC) as usize;
    let samples = notes.iter().flat_map(|&freq| {
        (0..per_note).map(move |i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            // A quick fade in avoids a click, then a long fade out.
            let envelope = (t / 0.01).min(1.0) * (-5.0 * t).exp();
            let tone = (2.0 * std::f32::consts::PI * freq * t).sin()
                + 0.3 * (4.0 * std::f32::consts::PI * freq * t).sin();
            (tone / 1.3 * envelope * VOLUME * i16::MAX as f32) as i16
        })
    });
    let data: Vec<u8> = samples.flat_map(i16::to_le_bytes).collect();

    let mut out = Vec::with_capacity(44 + data.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // bytes per sample
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// Plays without waiting. The sound must outlive the call, so each one is made once and kept.
pub fn play(chime: Chime) {
    static FOCUS: OnceLock<Vec<u8>> = OnceLock::new();
    static BREAK: OnceLock<Vec<u8>> = OnceLock::new();
    let cell = match chime {
        Chime::FocusDone => &FOCUS,
        Chime::BreakDone => &BREAK,
    };
    let sound = cell.get_or_init(|| wav(chime.notes()));
    // SAFETY: with SND_MEMORY the pointer is to WAV bytes, which live for the whole program.
    unsafe {
        let _ = PlaySoundW(
            PCWSTR(sound.as_ptr().cast()),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_a_valid_wav_of_the_right_length() {
        let bytes = wav(&[440.0, 880.0]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
        assert_eq!(bytes.len(), 44 + data_len);
        assert_eq!(data_len, 2 * (SAMPLE_RATE as f32 * NOTE_SEC) as usize * 2);
        let riff_len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff_len, bytes.len() - 8);
    }

    /// Plays both through the speakers: `cargo test -p monk-mode-app -- --ignored`.
    #[test]
    #[ignore]
    fn plays_both_chimes() {
        for chime in [Chime::FocusDone, Chime::BreakDone] {
            play(chime);
            std::thread::sleep(std::time::Duration::from_millis(1400));
        }
    }

    #[test]
    fn starts_and_ends_quietly_so_it_does_not_click() {
        let bytes = wav(&[440.0]);
        let sample = |i: usize| i16::from_le_bytes([bytes[44 + i * 2], bytes[45 + i * 2]]);
        let count = (bytes.len() - 44) / 2;
        assert!(sample(0).abs() < 100);
        assert!(sample(count - 1).abs() < 2_000);
        let loudest = (0..count).map(|i| sample(i).unsigned_abs()).max().unwrap();
        assert!(loudest > 5_000 && loudest < i16::MAX as u16);
    }
}
