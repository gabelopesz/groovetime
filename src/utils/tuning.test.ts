import { describe, expect, it } from "vitest";
import { BASS_4, BASS_5, DROP_D, HALF_STEP_5, TUNINGS } from "../data/tunings";
import { freqToNote, midiToFreq, noteToMidi } from "./note";
import { nearestString } from "./tuning";

describe("note", () => {
  it("converte nota <-> MIDI e frequência", () => {
    expect(noteToMidi("A", 4)).toBe(69);
    expect(noteToMidi("B", 0)).toBe(23);
    expect(midiToFreq(69, 440)).toBeCloseTo(440, 6);
    expect(midiToFreq(33, 440)).toBeCloseTo(55, 6);
  });

  it("A4 muda a nota/cents: 54 Hz é A1 ~0 cents com 432 e ~-32 com 440", () => {
    const a = freqToNote(54, 432)!;
    expect([a.name, a.octave]).toEqual(["A", 1]);
    expect(Math.abs(a.cents)).toBeLessThan(0.1);
    const b = freqToNote(54, 440)!;
    expect([b.name, b.octave]).toEqual(["A", 1]);
    expect(b.cents).toBeCloseTo(-31.77, 1);
  });

  it("rejeita entradas inválidas", () => {
    expect(freqToNote(0, 440)).toBeNull();
    expect(freqToNote(Number.NaN, 440)).toBeNull();
  });
});

describe("nearestString", () => {
  it("Drop D: D1 (36.71 Hz) seleciona a corda D1, não E1", () => {
    const t = nearestString(36.71, DROP_D.strings, 440)!;
    expect(DROP_D.strings[t.index]).toMatchObject({ note: "D", octave: 1 });
    expect(Math.abs(t.cents)).toBeLessThan(1);
  });

  it("no padrão de 4 cordas, 36.71 Hz está a ~200 cents de E1: fora do alcance", () => {
    expect(nearestString(36.71, BASS_4.strings, 440)).toBeNull();
  });

  it("cents são relativos à corda alvo e seguem o A4", () => {
    // A1 = 54 Hz com A4 = 432 → afinado; com A4 = 440 → ~-32 cents.
    expect(nearestString(54, BASS_4.strings, 432)!.cents).toBeCloseTo(0, 1);
    expect(nearestString(54, BASS_4.strings, 440)!.cents).toBeCloseTo(-31.77, 1);
  });

  it("5 cordas: B0 (30.87 Hz) seleciona a primeira corda", () => {
    const t = nearestString(30.87, BASS_5.strings, 440)!;
    expect(t.index).toBe(0);
    expect(Math.abs(t.cents)).toBeLessThan(1);
  });

  it("meio tom abaixo em 5 cordas: A#0 (29.14 Hz) fica acima do limite do YIN (28 Hz)", () => {
    const a0 = midiToFreq(noteToMidi("A#", 0), 440);
    expect(a0).toBeGreaterThan(28);
    const t = nearestString(a0, HALF_STEP_5.strings, 440)!;
    expect(t.index).toBe(0);
  });

  it("sem cordas (cromático) ou frequência inválida → null", () => {
    expect(nearestString(55, [], 440)).toBeNull();
    expect(nearestString(Number.NaN, BASS_4.strings, 440)).toBeNull();
  });

  it("todos os presets ficam acima de 28 Hz (limite do YIN)", () => {
    for (const tuning of TUNINGS) {
      for (const str of tuning.strings) {
        expect(midiToFreq(noteToMidi(str.note, str.octave), 440)).toBeGreaterThan(28);
      }
    }
  });
});
