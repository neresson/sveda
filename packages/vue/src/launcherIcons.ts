import {
  Bot,
  BotMessageSquare,
  Brain,
  CircleHelp,
  Gem,
  Heart,
  MessageCircle,
  MessageSquare,
  Rocket,
  Sparkles,
  Star,
  Zap,
  type LucideIcon,
} from 'lucide-vue-next';
import { VEDA_DEFAULT_LAUNCHER_ICON, VEDA_LAUNCHER_ICON_IDS, type VedaLauncherIconId } from './appearance';

export const vedaLauncherIconMap: Record<VedaLauncherIconId, LucideIcon> = {
  sparkles: Sparkles,
  'message-circle': MessageCircle,
  'message-square': MessageSquare,
  bot: Bot,
  'bot-message-square': BotMessageSquare,
  brain: Brain,
  zap: Zap,
  star: Star,
  heart: Heart,
  'circle-help': CircleHelp,
  rocket: Rocket,
  gem: Gem,
};

export const vedaLauncherIconComponent = (icon: string): LucideIcon => {
  if ((VEDA_LAUNCHER_ICON_IDS as readonly string[]).includes(icon)) {
    return vedaLauncherIconMap[icon as VedaLauncherIconId];
  }

  return vedaLauncherIconMap[VEDA_DEFAULT_LAUNCHER_ICON];
};
