export const HISTORY_DECIMATION_TARGET = 200;

export interface AliveSeriesPoint {
  generation: number;
  alive: number;
}

export const decimatedIndices = (
  length: number,
  target = HISTORY_DECIMATION_TARGET,
): number[] => {
  if (length <= 0) {
    return [];
  }
  if (length <= target) {
    return Array.from({ length }, (_, generation) => generation);
  }

  const stride = Math.ceil(length / target);
  const indices: number[] = [];
  for (let i = 0; i < length; i += stride) {
    indices.push(i);
  }
  if (indices[indices.length - 1] !== length - 1) {
    indices.push(length - 1);
  }
  return indices;
};

export const prepareSeries = (history: number[]): AliveSeriesPoint[] =>
  decimatedIndices(history.length).map((generation) => ({
    generation,
    alive: history[generation],
  }));
