import type { Config } from 'tailwindcss';

const vedaColor = (token: string) => `hsl(var(--veda-${token}))`;

export default {
  darkMode: ['class'],
  content: ['./src/**/*.{vue,ts}'],
  theme: {
    extend: {
      borderRadius: {
        lg: 'var(--veda-radius)',
        md: 'calc(var(--veda-radius) - 2px)',
        sm: 'calc(var(--veda-radius) - 4px)',
      },
      colors: {
        background: vedaColor('background'),
        foreground: vedaColor('foreground'),
        card: {
          DEFAULT: vedaColor('card'),
          foreground: vedaColor('card-foreground'),
        },
        popover: {
          DEFAULT: vedaColor('popover'),
          foreground: vedaColor('popover-foreground'),
        },
        primary: {
          DEFAULT: vedaColor('primary'),
          foreground: vedaColor('primary-foreground'),
        },
        secondary: {
          DEFAULT: vedaColor('secondary'),
          foreground: vedaColor('secondary-foreground'),
        },
        muted: {
          DEFAULT: vedaColor('muted'),
          foreground: vedaColor('muted-foreground'),
        },
        accent: {
          DEFAULT: vedaColor('accent'),
          foreground: vedaColor('accent-foreground'),
        },
        destructive: {
          DEFAULT: vedaColor('destructive'),
          foreground: vedaColor('destructive-foreground'),
        },
        border: vedaColor('border'),
        input: vedaColor('input'),
        ring: vedaColor('ring'),
        brand: {
          'primary-purple': vedaColor('brand'),
          'primary-purple-foreground': vedaColor('brand-foreground'),
        },
      },
    },
  },
  plugins: [require('tailwindcss-animate')],
} satisfies Config;
