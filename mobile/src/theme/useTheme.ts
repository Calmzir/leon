// The palette of the chosen appearance (Leon is dark unless told otherwise),
// and whether motion is wanted.
import { useEffect, useState } from 'react';
import { AccessibilityInfo, useColorScheme } from 'react-native';

import { useApp } from '@/store/app';

import { dark, light, motion, type Palette } from './tokens';

export function useIsDark(): boolean {
  const appearance = useApp((s) => s.appearance);
  const system = useColorScheme();
  return appearance === 'system' ? system !== 'light' : appearance !== 'light';
}

export function useTheme(): Palette {
  return useIsDark() ? dark : light;
}

/** Durations to animate with: all zero when the person asked for less motion. */
export function useMotion(): { quick: number; open: number } {
  const [reduced, setReduced] = useState(false);
  useEffect(() => {
    let alive = true;
    AccessibilityInfo.isReduceMotionEnabled().then((value) => {
      if (alive) setReduced(value);
    });
    const subscription = AccessibilityInfo.addEventListener('reduceMotionChanged', setReduced);
    return () => {
      alive = false;
      subscription.remove();
    };
  }, []);
  return reduced ? { quick: 0, open: 0 } : motion;
}
