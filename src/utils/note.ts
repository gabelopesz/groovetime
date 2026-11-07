// Conversões nota/frequência (espelha src-tauri/src/dsp/note.rs: sustenidos, A4 = MIDI 69).

export const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"] as const;

/** Número MIDI de uma nota (ex.: "A", 4 → 69). Devolve NaN se o nome for desconhecido. */
export function noteToMidi(name: string, octave: number): number {
  const idx = NOTE_NAMES.indexOf(name as (typeof NOTE_NAMES)[number]);
  return idx < 0 ? Number.NaN : (octave + 1) * 12 + idx;
}

export function midiToFreq(midi: number, a4Hz: number): number {
  return a4Hz * 2 ** ((midi - 69) / 12);
}

/** Desvio em cents de `freq` em relação a `target` (positivo = agudo). */
export function centsBetween(freq: number, target: number): number {
  return 1200 * Math.log2(freq / target);
}

export interface NoteReading {
  name: string;
  octave: number;
  midi: number;
  /** Desvio da nota mais próxima, em [-50, 50]. */
  cents: number;
}

export function freqToNote(freq: number, a4Hz: number): NoteReading | null {
  if (!(Number.isFinite(freq) && freq > 0 && Number.isFinite(a4Hz) && a4Hz > 0)) return null;
  const exact = 69 + 12 * Math.log2(freq / a4Hz);
  const midi = Math.round(exact);
  const idx = ((midi % 12) + 12) % 12;
  return {
    name: NOTE_NAMES[idx]!,
    octave: Math.floor(midi / 12) - 1,
    midi,
    cents: (exact - midi) * 100,
  };
}
