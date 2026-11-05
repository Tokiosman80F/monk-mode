// Design tokens from PRD 6.1.
const radii = {
  "--rounded-box": "1rem",
  "--rounded-btn": "0.75rem",
  "--rounded-badge": "1.5rem",
};

export const monkLight = {
  "base-100": "#FAF8F3",
  "base-200": "#F2EEE6",
  "base-300": "#E6E0D4",
  "base-content": "#2B2A27",
  primary: "#C8643B",
  "primary-content": "#FFFFFF",
  secondary: "#7A8B6F",
  neutral: "#6B675E",
  success: "#5E8C61",
  warning: "#D9A441",
  error: "#B5473A",
};

export const monkDark = {
  "base-100": "#1F1E1B",
  "base-200": "#2A2925",
  "base-300": "#3A3833",
  "base-content": "#ECE8DF",
  primary: "#D97A52",
  "primary-content": "#1F1E1B",
  secondary: "#93A487",
  neutral: "#A39E93",
  success: "#78A57B",
  warning: "#E2B65C",
  error: "#CF6456",
};

export const daisyThemes = [
  { "monk-light": { ...monkLight, ...radii } },
  { "monk-dark": { ...monkDark, ...radii } },
];
