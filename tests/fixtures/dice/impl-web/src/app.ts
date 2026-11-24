export function title(): string {
  return "Dice Anything";
}

export function roll(sides: number, random: () => number): number {
  return 1 + Math.floor(random() * sides);
}
