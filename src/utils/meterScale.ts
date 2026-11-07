// Escala não linear do meter: mais resolução perto do topo (0 dBFS).

export const METER_MIN_DB = -60;
export const METER_MAX_DB = 0;
/** Limiar do alerta PEAK (mesmo valor do backend). */
export const PEAK_ALERT_DBFS = -3;

/** Pontos (dBFS -> posição 0..1) interpolados linearmente. */
const POINTS: ReadonlyArray<readonly [number, number]> = [
  [-60, 0],
  [-48, 0.12],
  [-36, 0.24],
  [-24, 0.4],
  [-18, 0.53],
  [-12, 0.66],
  [-6, 0.82],
  [-3, 0.91],
  [0, 1],
];

export function dbToPosition(db: number): number {
  if (!Number.isFinite(db) || db <= METER_MIN_DB) return 0;
  if (db >= METER_MAX_DB) return 1;
  for (let i = 1; i < POINTS.length; i++) {
    const [d1, p1] = POINTS[i]!;
    if (db <= d1) {
      const [d0, p0] = POINTS[i - 1]!;
      return p0 + ((db - d0) / (d1 - d0)) * (p1 - p0);
    }
  }
  return 1;
}

/** Marcações exibidas sob o meter. */
export const METER_TICKS = [-60, -48, -24, -12, -6, -3, 0] as const;
