export interface Quote {
  text: string;
  by: string;
}

const QUOTES: Quote[] = [
  {
    text: "The journey of a thousand miles begins with a single step.",
    by: "Lao Tzu",
  },
  { text: "Begin at once to live.", by: "Seneca" },
  {
    text: "It is not that we have a short time to live, but that we waste a lot of it.",
    by: "Seneca",
  },
  {
    text: "Waste no more time arguing what a good man should be. Be one.",
    by: "Marcus Aurelius",
  },
  {
    text: "No man is free who is not master of himself.",
    by: "Epictetus",
  },
  {
    text: "First say to yourself what you would be, and then do what you have to do.",
    by: "Epictetus",
  },
  { text: "Dripping water hollows out stone.", by: "Ovid" },
  { text: "Well begun is half done.", by: "Proverb" },
  { text: "Little by little, one travels far.", by: "Proverb" },
  { text: "Fall seven times, stand up eight.", by: "Japanese proverb" },
  {
    text: "The best time to plant a tree was twenty years ago. The second best time is now.",
    by: "Proverb",
  },
  { text: "Slow and steady wins the race.", by: "Aesop" },
];

/** A new quote each day, and the same one all day. */
export function quoteFor(date: Date): Quote {
  const days = Math.floor(
    Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()) / 86_400_000,
  );
  return QUOTES[days % QUOTES.length];
}
