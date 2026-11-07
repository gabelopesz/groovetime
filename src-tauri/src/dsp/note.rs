//! Conversao frequencia <-> nota/cents. A4 e sempre parametro.

pub const DEFAULT_A4_HZ: f32 = 440.0;

const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteInfo {
    pub name: &'static str,
    pub octave: i32,
    /// Numero MIDI da nota mais proxima (A4 = 69).
    pub midi: i32,
    /// Desvio em cents da nota mais proxima, em [-50, 50].
    pub cents: f32,
    /// Frequencia exata da nota mais proxima.
    pub target_hz: f32,
}

/// Frequencia de uma nota MIDI dado A4.
pub fn note_to_freq(midi: i32, a4_hz: f32) -> f32 {
    a4_hz * 2f32.powf((midi - 69) as f32 / 12.0)
}

/// Nota mais proxima de `freq_hz`. `None` se a frequencia ou A4 forem invalidos.
pub fn freq_to_note(freq_hz: f32, a4_hz: f32) -> Option<NoteInfo> {
    if !(freq_hz.is_finite() && freq_hz > 0.0 && a4_hz.is_finite() && a4_hz > 0.0) {
        return None;
    }
    let exact = 69.0 + 12.0 * (freq_hz / a4_hz).log2();
    let midi = exact.round() as i32;
    let cents = (exact - midi as f32) * 100.0;
    Some(NoteInfo {
        name: NAMES[midi.rem_euclid(12) as usize],
        octave: midi.div_euclid(12) - 1,
        midi,
        cents,
        target_hz: note_to_freq(midi, a4_hz),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_is_a4() {
        let n = freq_to_note(440.0, 440.0).unwrap();
        assert_eq!((n.name, n.octave, n.midi), ("A", 4, 69));
        assert!(n.cents.abs() < 1e-3);
    }

    #[test]
    fn bass_strings() {
        for (hz, name, oct) in [
            (30.87, "B", 0),
            (41.20, "E", 1),
            (55.00, "A", 1),
            (73.42, "D", 2),
            (98.00, "G", 2),
            (130.81, "C", 3),
        ] {
            let n = freq_to_note(hz, 440.0).unwrap();
            assert_eq!((n.name, n.octave), (name, oct), "{hz}");
            assert!(n.cents.abs() < 1.0, "{hz}: {}", n.cents);
        }
    }

    #[test]
    fn a4_is_a_parameter() {
        let n = freq_to_note(432.0, 432.0).unwrap();
        assert_eq!((n.name, n.octave), ("A", 4));
        let n = freq_to_note(440.0, 432.0).unwrap();
        assert!((n.cents - 31.77).abs() < 0.1, "{}", n.cents);
    }

    #[test]
    fn cents_sign_and_invalid() {
        assert!(freq_to_note(55.0 * 1.01, 440.0).unwrap().cents > 0.0);
        assert!(freq_to_note(55.0 * 0.99, 440.0).unwrap().cents < 0.0);
        assert!(freq_to_note(0.0, 440.0).is_none());
        assert!(freq_to_note(f32::NAN, 440.0).is_none());
    }
}
