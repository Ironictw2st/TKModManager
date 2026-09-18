/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: "rgb(var(--bg) / <alpha-value>)",
        panel: "rgb(var(--panel) / <alpha-value>)",
        panelAlt: "rgb(var(--panel-alt) / <alpha-value>)",
        panelHeader: "rgb(var(--panel-header) / <alpha-value>)",
        edge: "rgb(var(--edge) / <alpha-value>)",
        accent: "rgb(var(--accent) / <alpha-value>)",
        hover: "rgb(var(--hover) / <alpha-value>)",
        button: "rgb(var(--button) / <alpha-value>)",
        buttonHover: "rgb(var(--button-hover) / <alpha-value>)",
        sunken: "rgb(var(--sunken) / <alpha-value>)",
        text: "rgb(var(--text) / <alpha-value>)",
        textMuted: "rgb(var(--text-muted) / <alpha-value>)",
        selected: "rgb(var(--selected) / <alpha-value>)",
        ok: "rgb(var(--ok) / <alpha-value>)",
        warn: "rgb(var(--warn) / <alpha-value>)",
        danger: "rgb(var(--danger) / <alpha-value>)",
        se: "rgb(var(--se) / <alpha-value>)",
      },
    },
  },
  plugins: [],
};
