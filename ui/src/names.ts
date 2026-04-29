const ADJECTIVES = [
  "Fuzzy",
  "Spicy",
  "Grumpy",
  "Wobbly",
  "Sneaky",
  "Bouncy",
  "Fluffy",
  "Zesty",
  "Clumsy",
  "Sassy",
  "Wiggly",
  "Cheeky",
  "Dizzy",
  "Jolly",
  "Wacky",
  "Peppy",
  "Goofy",
  "Nifty",
  "Perky",
  "Zippy",
];

const ANIMALS = [
  "Badger",
  "Narwhal",
  "Capybara",
  "Penguin",
  "Platypus",
  "Axolotl",
  "Quokka",
  "Wombat",
  "Meerkat",
  "Pangolin",
  "Tapir",
  "Manatee",
  "Binturong",
  "Fossa",
  "Numbat",
  "Echidna",
  "Kinkajou",
  "Potoo",
  "Blobfish",
  "Tardigrade",
];

export function generateSeriousName(): string {
  const adjective = ADJECTIVES[Math.floor(Math.random() * ADJECTIVES.length)]!;
  const animal = ANIMALS[Math.floor(Math.random() * ANIMALS.length)]!;
  return `${adjective}${animal}`;
}
