// Afinações como dados. Notas escritas como o backend as envia (sustenidos, notação anglo).

export interface TuningString {
  /** Rótulo curto exibido na referência (ex.: "E", "D#"). */
  name: string;
  /** Nome da nota como enviado pelo backend (sustenidos). */
  note: string;
  octave: number;
}

export interface Tuning {
  id: string;
  label: string;
  /** Vazio = modo cromático (usa a nota/cents do backend). */
  strings: TuningString[];
}

const s = (note: string, octave: number): TuningString => ({ name: note, note, octave });

export const CHROMATIC: Tuning = { id: "chromatic", label: "Cromático (qualquer nota)", strings: [] };

export const BASS_4: Tuning = {
  id: "bass-4",
  label: "4 cordas (E A D G)",
  strings: [s("E", 1), s("A", 1), s("D", 2), s("G", 2)],
};

export const BASS_5: Tuning = {
  id: "bass-5",
  label: "5 cordas (B E A D G)",
  strings: [s("B", 0), s("E", 1), s("A", 1), s("D", 2), s("G", 2)],
};

export const BASS_6: Tuning = {
  id: "bass-6",
  label: "6 cordas (B E A D G C)",
  strings: [s("B", 0), s("E", 1), s("A", 1), s("D", 2), s("G", 2), s("C", 3)],
};

export const DROP_D: Tuning = {
  id: "drop-d",
  label: "Drop D (D A D G)",
  strings: [s("D", 1), s("A", 1), s("D", 2), s("G", 2)],
};

/** Meio tom abaixo: E♭ A♭ D♭ G♭ (escrito com sustenidos, como o resto da UI). */
export const HALF_STEP_4: Tuning = {
  id: "half-step-4",
  label: "4 cordas ½ tom abaixo (D# G# C# F#)",
  strings: [s("D#", 1), s("G#", 1), s("C#", 2), s("F#", 2)],
};

export const HALF_STEP_5: Tuning = {
  id: "half-step-5",
  label: "5 cordas ½ tom abaixo (A# D# G# C# F#)",
  strings: [s("A#", 0), s("D#", 1), s("G#", 1), s("C#", 2), s("F#", 2)],
};

export const TUNINGS: Tuning[] = [BASS_4, BASS_5, BASS_6, DROP_D, HALF_STEP_4, HALF_STEP_5, CHROMATIC];

export const DEFAULT_TUNING: Tuning = BASS_4;

export function tuningById(id: string): Tuning {
  return TUNINGS.find((t) => t.id === id) ?? DEFAULT_TUNING;
}

/** Tolerância (em cents) da zona "afinado". */
export const IN_TUNE_CENTS = 3;
