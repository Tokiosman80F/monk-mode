import daisyui from "daisyui";
import plugin from "tailwindcss/plugin";
import { daisyThemes } from "./src/theme";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["Inter", "sans-serif"],
        serif: ['"Source Serif 4"', "serif"],
      },
      keyframes: {
        breathe: {
          "0%, 100%": { transform: "scale(1)", opacity: "0.85" },
          "50%": { transform: "scale(1.04)", opacity: "1" },
        },
        "fade-up": {
          from: { transform: "translateY(6px)", opacity: "0" },
          to: { transform: "translateY(0)", opacity: "1" },
        },
        "fade-in": {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
      },
      animation: {
        breathe: "breathe 4s ease-in-out infinite",
        "fade-up": "fade-up 250ms ease-out both",
        "fade-in": "fade-in 150ms ease-out both",
      },
    },
  },
  plugins: [
    daisyui,
    plugin(({ addBase }) =>
      addBase({
        body: { fontSize: "15px", lineHeight: "1.6" },
        ".btn": { transitionDuration: "150ms" },
        ".progress::-webkit-progress-value": {
          transition: "inline-size 600ms ease-out, width 600ms ease-out",
        },
        "@media (prefers-reduced-motion: reduce)": {
          "*, ::before, ::after": {
            animation: "none !important",
            transition: "none !important",
          },
        },
      }),
    ),
  ],
  daisyui: { themes: daisyThemes, darkTheme: "monk-dark", logs: false },
};
