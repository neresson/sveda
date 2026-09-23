import type { VariantProps } from 'class-variance-authority'
import { cva } from 'class-variance-authority'

export { default as Button } from './Button.vue'

export const buttonVariants = cva(
  'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-[var(--sveda-radius)] text-sm font-medium ring-offset-card transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0',
  {
    variants: {
      variant: {
        default: 'text-brand-primary-purple-foreground bg-brand-primary-purple',
        destructive: 'bg-destructive text-destructive-foreground',
        outline: 'border-[1px] bg-transparent border-brand-primary-purple text-brand-primary-purple',
        secondary: 'bg-transparent border-[1px] border-brand-primary-purple text-brand-primary-purple',
        ghost: 'text-muted-foreground',
        ghostTransparent: 'bg-transparent text-muted-foreground',
        link: 'text-brand-primary-purple underline-offset-4',
        violet: 'text-brand-primary-purple-foreground bg-brand-primary-purple',
        main: 'text-brand-primary-purple-foreground bg-brand-primary-purple',
      },
      size: {
        default: 'h-10 px-4 py-2 ',
        sm: 'min-h-9  px-3 max-sm:h-auto max-sm:w-auto max-sm:text-wrap',
        lg: 'h-11 px-8 max-sm:h-7 max-sm:text-xs',
        icon: 'h-10 w-10',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  },
)

export const buttonHoverStyles: Record<
  NonNullable<VariantProps<typeof buttonVariants>['variant']>,
  string
> = {
  default: 'opacity-90',
  destructive: 'bg-destructive/90',
  outline: 'bg-foreground/5 dark:bg-foreground/5 border-brand-primary-purple text-brand-primary-purple',
  secondary: 'bg-foreground/5 dark:bg-foreground/5 border-brand-primary-purple text-brand-primary-purple',
  ghost: 'bg-muted text-foreground',
  ghostTransparent: 'bg-transparent text-foreground',
  link: 'underline',
  violet: 'opacity-90',
  main: 'opacity-90',
};

export type ButtonVariants = VariantProps<typeof buttonVariants>
