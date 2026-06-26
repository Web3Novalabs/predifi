import type { Config } from "tailwindcss";

const config: Config = {
    darkMode: ["class"],
    content: [
        "./app/**/*.{js,ts,jsx,tsx,mdx}",
        "./components/**/*.{js,ts,jsx,tsx,mdx}",
        "./lib/**/*.{js,ts,jsx,tsx,mdx}",
    ],
    theme: {
        extend: {
            colors: {
                background: "var(--background)",
                foreground: "var(--foreground)",
                card: {
                    DEFAULT: "hsl(var(--card))",
                    foreground: "hsl(var(--card-foreground))",
                },
                popover: {
                    DEFAULT: "hsl(var(--popover))",
                    foreground: "hsl(var(--popover-foreground))",
                },
                primary: {
                    DEFAULT: "hsl(var(--primary))",
                    foreground: "hsl(var(--primary-foreground))",
                },
                secondary: {
                    DEFAULT: "hsl(var(--secondary))",
                    foreground: "hsl(var(--secondary-foreground))",
                },
                muted: {
                    DEFAULT: "hsl(var(--muted))",
                    foreground: "hsl(var(--muted-foreground))",
                },
                accent: {
                    DEFAULT: "hsl(var(--accent))",
                    foreground: "hsl(var(--accent-foreground))",
                },
                destructive: {
                    DEFAULT: "hsl(var(--destructive))",
                    foreground: "hsl(var(--destructive-foreground))",
                },
                success: {
                    DEFAULT: "hsl(var(--success))",
                    foreground: "hsl(var(--success-foreground))",
                },
                warning: {
                    DEFAULT: "hsl(var(--warning))",
                    foreground: "hsl(var(--warning-foreground))",
                },
                info: {
                    DEFAULT: "hsl(var(--info))",
                    foreground: "hsl(var(--info-foreground))",
                },
                border: "hsl(var(--border))",
                input: "hsl(var(--input))",
                ring: "hsl(var(--ring))",
                chart: {
                    "1": "hsl(var(--chart-1))",
                    "2": "hsl(var(--chart-2))",
                    "3": "hsl(var(--chart-3))",
                    "4": "hsl(var(--chart-4))",
                    "5": "hsl(var(--chart-5))",
                },
            },
            borderRadius: {
                lg: "var(--radius)",
                md: "calc(var(--radius) - 2px)",
                sm: "calc(var(--radius) - 4px)",
            },
            keyframes: {
                // Page-level fade-in (used by settings panel transitions)
                fadeIn: {
                    from: { opacity: "0", transform: "translateY(10px)" },
                    to:   { opacity: "1", transform: "translateY(0)" },
                },
                // Toast enter: slide in from the right edge
                "toast-in": {
                    from: { opacity: "0", transform: "translateX(calc(100% + 1rem))" },
                    to:   { opacity: "1", transform: "translateX(0)" },
                },
                // Toast exit: slide back out, then collapse height so the stack closes up
                "toast-out": {
                    "0%":   { opacity: "1", transform: "translateX(0)", maxHeight: "200px", marginBottom: "0.5rem" },
                    "60%":  { opacity: "0", transform: "translateX(calc(100% + 1rem))" },
                    "100%": { opacity: "0", transform: "translateX(calc(100% + 1rem))", maxHeight: "0", marginBottom: "0" },
                },
                // Progress bar drains from full width to zero
                "toast-progress": {
                    from: { transform: "scaleX(1)" },
                    to:   { transform: "scaleX(0)" },
                },
            },
            animation: {
                "fade-in":        "fadeIn 0.5s ease-out forwards",
                "toast-in":       "toast-in  0.35s cubic-bezier(0.16, 1, 0.3, 1) forwards",
                "toast-out":      "toast-out 0.4s  cubic-bezier(0.4,  0, 1,   1) forwards",
                "toast-progress": "toast-progress linear forwards",
            },
        },
    },
    plugins: [],
};
export default config;
