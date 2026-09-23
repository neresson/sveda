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
} from 'lucide-solid';
import type { Component, JSX } from 'solid-js';
import { SVEDA_DEFAULT_LAUNCHER_ICON, SVEDA_LAUNCHER_ICON_IDS, type SvedaLauncherIconId } from './appearance';

type LucideIcon = Component<JSX.SvgSVGAttributes<SVGSVGElement>>;

export const svedaLauncherIconMap: Record<SvedaLauncherIconId, LucideIcon> = {
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

export const svedaLauncherIconComponent = (icon: string): LucideIcon => {
  if ((SVEDA_LAUNCHER_ICON_IDS as readonly string[]).includes(icon)) {
    return svedaLauncherIconMap[icon as SvedaLauncherIconId];
  }

  return svedaLauncherIconMap[SVEDA_DEFAULT_LAUNCHER_ICON];
};
