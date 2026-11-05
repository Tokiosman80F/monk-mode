import daisyui from "daisyui";
import plugin from "tailwindcss/plugin";
import { daisyThemes } from "./src/theme";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["Inter", "sans-serif"],
        display: ['"Bricolage Grotesque"', "sans-serif"],
      },
      dropShadow: {
        glow: "0 0 6px rgb(255 178 36 / 0.55)",
      },
      boxShadow: {
        glow: "0 0 18px rgb(255 178 36 / 0.35)",
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
        "xp-pop": {
          "0%": { transform: "translateY(4px)", opacity: "0" },
          "20%": { transform: "translateY(0)", opacity: "1" },
          "80%": { transform: "translateY(-2px)", opacity: "1" },
          "100%": { transform: "translateY(-10px)", opacity: "0" },
        },
      },
      animation: {
        breathe: "breathe 4s ease-in-out infinite",
        "fade-up": "fade-up 250ms ease-out both",
        "fade-in": "fade-in 150ms ease-out both",
        "xp-pop": "xp-pop 1500ms ease-out both",
      },
    },
  },
  plugins: [
    daisyui,
    plugin(({ addBase }) =>
      addBase({
        body: { fontSize: "14px", lineHeight: "1.55" },
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
