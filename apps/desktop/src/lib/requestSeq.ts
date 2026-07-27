/** Monotonic request token so only the latest async load may commit state. */
export function createRequestSeq() {
  let current = 0;
  return {
    begin(): number {
      current += 1;
      return current;
    },
    isCurrent(token: number): boolean {
      return token === current;
    },
  };
}
