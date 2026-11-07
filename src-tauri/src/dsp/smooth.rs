//! Estabilizacao da leitura de pitch: mediana das ultimas estimativas + hold curto.

const HISTORY: usize = 5;

#[derive(Debug, Clone)]
pub struct PitchSmoother {
    history: [f32; HISTORY],
    len: usize,
    pos: usize,
    misses: usize,
    last: Option<f32>,
    max_misses: usize,
}

impl PitchSmoother {
    /// `max_misses`: quantas leituras vazias consecutivas ainda mantem a ultima nota.
    pub fn new(max_misses: usize) -> Self {
        Self { history: [0.0; HISTORY], len: 0, pos: 0, misses: 0, last: None, max_misses }
    }

    pub fn reset(&mut self) {
        self.len = 0;
        self.pos = 0;
        self.misses = 0;
        self.last = None;
    }

    /// Entrega uma nova estimativa (ou `None`) e recebe a leitura estabilizada.
    pub fn update(&mut self, estimate: Option<f32>) -> Option<f32> {
        match estimate {
            Some(f) if f.is_finite() && f > 0.0 => {
                self.misses = 0;
                self.history[self.pos] = f;
                self.pos = (self.pos + 1) % HISTORY;
                self.len = (self.len + 1).min(HISTORY);
                self.last = Some(self.median());
            }
            _ => {
                self.misses += 1;
                if self.misses > self.max_misses {
                    self.reset();
                }
            }
        }
        self.last
    }

    fn median(&self) -> f32 {
        let mut buf = self.history;
        let slice = &mut buf[..self.len];
        slice.sort_by(f32::total_cmp);
        slice[self.len / 2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_rejects_outlier() {
        let mut s = PitchSmoother::new(3);
        for f in [55.0, 55.1, 110.0, 55.0] {
            s.update(Some(f));
        }
        let m = s.update(Some(55.05)).unwrap();
        assert!((m - 55.05).abs() < 0.1, "{m}");
    }

    #[test]
    fn holds_then_clears_on_silence() {
        let mut s = PitchSmoother::new(2);
        s.update(Some(41.2));
        assert!(s.update(None).is_some());
        assert!(s.update(None).is_some());
        assert!(s.update(None).is_none());
        // Depois de limpar, o historico recomeca.
        assert_eq!(s.update(Some(98.0)), Some(98.0));
    }

    #[test]
    fn follows_note_change() {
        let mut s = PitchSmoother::new(2);
        for _ in 0..5 {
            s.update(Some(55.0));
        }
        let mut last = 0.0;
        for _ in 0..3 {
            last = s.update(Some(73.4)).unwrap();
        }
        assert!((last - 73.4).abs() < 0.1);
    }
}
