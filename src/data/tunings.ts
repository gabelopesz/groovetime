// Afinações como dados (o passo 11 adiciona presets sobre esta estrutura).

export interface TuningString {
  /** Rótulo curto exibido na referência (ex.: "E"). */
  name: string;
  /** Nome da nota como enviado pelo backend (sustenidos, notação anglo). */
  note: string;
  octave: number;
}

export interface Tuning {
  id: string;
  label: string;
  strings: TuningString[];
}

export const BASS_4: Tuning = {
  id: "bass-4",
  label: "Baixo 4 cordas (E A D G)",
  strings: [
    { name: "E", note: "E", octave: 1 },
    { name: "A", note: "A", octave: 1 },
    { name: "D", note: "D", octave: 2 },
    { name: "G", note: "G", octave: 2 },
  ],
};

export const DEFAULT_TUNING: Tuning = BASS_4;

/** Tolerância (em cents) da zona "afinado". */
export const IN_TUNE_CENTS = 3;
