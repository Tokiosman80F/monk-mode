// Design tokens from PRD 6.1: black and ivory. Colour carries meaning only: one hue per quadrant,
// jade green for focus, gold for rewards.
const radii = {
  "--rounded-box": "1.125rem",
  "--rounded-btn": "0.75rem",
  "--rounded-badge": "1.5rem",
};

export const monkDark = {
  "base-100": "#000000",
  "base-200": "#101211",
  "base-300": "#2A2F2C",
  "base-content": "#F1EDE2",
  primary: "#F1EDE2",
  "primary-content": "#000000",
  secondary: "#6FDDB0",
  "secondary-content": "#000000",
  accent: "#F2BE5C",
  "accent-content": "#000000",
  neutral: "#A3B3AC",
  success: "#6FDDB0",
  warning: "#F2BE5C",
  error: "#FF8A6B",
};

export const quadrantColors = {
  "q-do": "#FF8A6B",
  "q-schedule": "#6FDDB0",
  "q-delegate": "#8FB8FF",
  "q-drop": "#A3B3AC",
};

export const daisyThemes = [{ "monk-dark": { ...monkDark, ...radii } }];
