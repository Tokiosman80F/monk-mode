// Design tokens from PRD 6.1. Saffron, the colour of monks' robes, on deep ink.
const radii = {
  "--rounded-box": "0.875rem",
  "--rounded-btn": "0.625rem",
  "--rounded-badge": "1.5rem",
};

export const monkDark = {
  "base-100": "#0C0E14",
  "base-200": "#14171F",
  "base-300": "#272B37",
  "base-content": "#EEF0F6",
  primary: "#FFB224",
  "primary-content": "#1B1203",
  secondary: "#FFD27A",
  neutral: "#9CA3B5",
  success: "#4ADE9B",
  warning: "#FFB224",
  error: "#FF6B5E",
};

export const daisyThemes = [{ "monk-dark": { ...monkDark, ...radii } }];
