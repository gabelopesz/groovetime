import type { TuningString } from "../data/tunings";
import { centsBetween, midiToFreq, noteToMidi } from "./note";

/** Acima disto a nota está "longe demais" de qualquer corda do preset. */
export const MAX_STRING_DISTANCE_CENTS = 100;

export interface StringTarget {
  /** Índice da corda alvo em `strings`. */
  index: number;
  /** Desvio em cents em relação à corda alvo (positivo = agudo). */
  cents: number;
  targetHz: number;
}

/**
 * Corda do preset mais próxima de `freq` (em cents) e o desvio em relação a ela.
 * `null` se não houver cordas ou a nota estiver além de `MAX_STRING_DISTANCE_CENTS`.
 */
export function nearestString(
  freq: number,
  strings: readonly TuningString[],
  a4Hz: number,
): StringTarget | null {
  if (!(Number.isFinite(freq) && freq > 0)) return null;
  let best: StringTarget | null = null;
  strings.forEach((s, index) => {
    const midi = noteToMidi(s.note, s.octave);
    if (Number.isNaN(midi)) return;
    const targetHz = midiToFreq(midi, a4Hz);
    const cents = centsBetween(freq, targetHz);
    if (best === null || Math.abs(cents) < Math.abs(best.cents)) best = { index, cents, targetHz };
  });
  const found = best as StringTarget | null;
  return found && Math.abs(found.cents) <= MAX_STRING_DISTANCE_CENTS ? found : null;
}
