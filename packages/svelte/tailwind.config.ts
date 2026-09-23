import type { Config } from 'tailwindcss';

const svedaColor = (token: string) => `hsl(var(--sveda-${token}))`;

export default {
  darkMode: ['class'],
  content: ['./src/**/*.{svelte,ts}'],
  theme: {
    extend: {
      borderRadius: {
        lg: 'var(--sveda-radius)',
        md: 'calc(var(--sveda-radius) - 2px)',
        sm: 'calc(var(--sveda-radius) - 4px)',
      },
      colors: {
        background: svedaColor('background'),
        foreground: svedaColor('foreground'),
        card: {
          DEFAULT: svedaColor('card'),
          foreground: svedaColor('card-foreground'),
        },
        popover: {
          DEFAULT: svedaColor('popover'),
          foreground: svedaColor('popover-foreground'),
        },
        primary: {
          DEFAULT: svedaColor('primary'),
          foreground: svedaColor('primary-foreground'),
        },
        secondary: {
          DEFAULT: svedaColor('secondary'),
          foreground: svedaColor('secondary-foreground'),
        },
        muted: {
          DEFAULT: svedaColor('muted'),
          foreground: svedaColor('muted-foreground'),
        },
        accent: {
          DEFAULT: svedaColor('accent'),
          foreground: svedaColor('accent-foreground'),
        },
        destructive: {
          DEFAULT: svedaColor('destructive'),
          foreground: svedaColor('destructive-foreground'),
        },
        border: svedaColor('border'),
        input: svedaColor('input'),
        ring: svedaColor('ring'),
        brand: {
          'primary-purple': svedaColor('brand'),
          'primary-purple-foreground': svedaColor('brand-foreground'),
        },
      },
    },
  },
  plugins: [require('tailwindcss-animate')],
} satisfies Config;
