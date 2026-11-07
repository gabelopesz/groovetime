# Step 3: Biblioteca DSP pura: pico dBFS, decimação e YIN

**Status:** pending
**Depends on:** 1

## Goal

Implementar, sem nenhuma dependência de áudio/Tauri, os algoritmos de DSP com testes: medidor de pico em dBFS (com hold/decay), decimador com filtro anti-aliasing e detector de pitch YIN otimizado para graves (até B0 ≈ 30.87 Hz), mais a conversão frequência → nota/cents.

## Scope

**In scope:**
- `src-tauri/src/dsp/mod.rs`, `dsp/peak.rs`, `dsp/decimate.rs`, `dsp/yin.rs`, `dsp/note.rs`.
- **Peak**: `PeakMeter` que processa blocos `&[f32]`, mantém o máximo `|x|` do intervalo, converte para dBFS (`20*log10(max)`, com piso, ex. -120 dBFS para silêncio), e fornece `peak_hold` com decay (ex.: segura ~1 s, cai X dB/s). Sinalizar `clip`/`over_threshold` quando ≥ -3 dBFS (threshold configurável, constante `PEAK_ALERT_DBFS = -3.0`).
- **Decimator**: fator inteiro (ex.: 192k/24 = 8 kHz, ou 48k/6 = 8 kHz — calcular fator a partir da sample rate real), filtro passa-baixa (biquads Butterworth em cascata ou FIR) com corte ~1–2 kHz. Estado pré-alocado, `process` sem alocação.
- **YIN**: estrutura `Yin` com buffers pré-alocados no construtor (`new(sample_rate, window_size, min_freq, max_freq)`), `detect(&mut self, frame: &[f32]) -> Option<PitchEstimate { freq_hz, clarity/confidence }>`. Etapas: difference function, cumulative mean normalized difference, threshold absoluto (~0.10–0.15), interpolação parabólica. `min_freq` ~ 28 Hz (margem abaixo de B0), `max_freq` ~ 500 Hz (confirmar). Gate de nível: não detectar se RMS abaixo de um limiar — o limiar deve ser um parâmetro mutável em runtime (`set_gate_dbfs`), pois o passo 10 o expõe ao usuário.
- **Note**: `freq → (nome da nota, oitava, cents)` com A4 recebido como parâmetro (default 440 Hz) — nunca hardcoded, pois o passo 11 o torna ajustável pela UI.
- Testes com senoides e sinais ricos em harmônicos (ex.: dente de serra/soma de harmônicos, que é mais parecido com corda de baixo) para B0 30.87, E1 41.20, A1 55.00, D2 73.42, G2 98.00, C3 130.81 — erro < ~1 cent em sinal limpo; e teste de que não há "octave error" típico (detectar o dobro da frequência).
- Benchmark simples ou teste de tempo garantindo que um `detect` cabe com folga no orçamento (rodar YIN talvez a cada ~16–30 ms).

**Out of scope:**
- Threads, ring buffer, callback (passos 4–5).
- Atenuação Ativo/Passivo (é aplicada na captura, passo 4).
- Smoothing/estabilização de leitura para UI (passo 5 decide mediana/EMA sobre os resultados).

## Context

- Janela YIN: para detectar período τ_max = fs_decimada / min_freq, a janela precisa de ≥ 2·τ_max. A 8 kHz e 28 Hz: τ_max ≈ 286 → janela ≥ 572 amostras (usar 1024 ≈ 128 ms). Sem decimar a 192 kHz seriam ~13.7k amostras com custo O(N²) — inviável a 60 Hz. Por isso decimação é obrigatória.
- Latência de afinação ~100–150 ms em B0 é aceitável e inerente à física (poucos ciclos).
- Opcional de otimização: difference function via FFT (crate `realfft`) — só se o benchmark mostrar necessidade; a 1024 amostras o direto O(N·τ) é ~300k ops, ok.
- Ataque de nota de baixo tem transiente forte e harmônicos > fundamental em cordas graves (B0 tem fundamental fraco em muitos baixos/captadores) — o YIN com threshold lida razoavelmente, mas testar com sinal sintético de fundamental atenuado.

## Open questions / ambiguities

- Faixa de frequência máxima: só cordas soltas/notas até ~G3 (~200 Hz) ou suportar regiões agudas do braço / harmônicos (até ~1 kHz)? Afeta a taxa decimada e o filtro.
- Usar crate pronta (`pitch-detection`, `pitch_detector`) ou implementação própria? O plano assume implementação própria (controle de alocação e otimização para graves); confirmar.
- Valores de hold/decay do peak (ex.: hold 1.5 s, release 20 dB/s?) — relevante para "replicar o LED PEAK" da placa.

## Acceptance criteria

- [ ] `cargo test dsp` passa com todos os casos de nota listados (erro < 1 cent em senoide; < 3 cents em sinal harmônico com fundamental atenuado).
- [ ] Teste de silêncio/ruído baixo retorna `None` em YIN e ~-120 dBFS no peak.
- [ ] Teste de pico: senoide amplitude 0.708 (~-3 dBFS) dispara `over_threshold`; 0.5 (~-6 dBFS) não.
- [ ] `Yin::detect`, `Decimator::process` e `PeakMeter::process` não alocam (buffers criados no construtor; revisar código).
- [ ] Clippy limpo, sem `unwrap`/`expect` fora de testes.
