// A lion of the Den: one frame of its strip, the frame moving with the beat
// of the screen. The strips are the desktop's pixel art, enlarged without
// smoothing (see `denSheets.ts`).
import { Image, View } from 'react-native';

import { framesOf, seedOf } from '@/chat/den';
import type { DenCub } from '@/link/types';

import { BODIES, SHEETS, STYLES } from './denSheets';

/** Frames in a strip. */
const FRAMES = 10;

export function Lion({
  cub,
  width,
  beat,
}: {
  cub: DenCub;
  /** The width of the lion; it is twice as tall. */
  width: number;
  /** Counts up while the den is in front; still when motion is reduced. */
  beat: number;
}) {
  const seed = seedOf(cub);
  const sheets = SHEETS[cub.agent] ?? SHEETS.other;
  const sheet = sheets?.[(seed % BODIES) * STYLES + (Math.floor(seed / BODIES) % STYLES)];
  const frames = framesOf(cub.state);
  // Lions do not move in step: each starts its round somewhere else.
  const frame = frames[(beat + seed) % frames.length] ?? 0;
  if (!sheet) return <View style={{ width, height: width * 2 }} />;
  return (
    <View
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      style={{ width, height: width * 2, overflow: 'hidden' }}>
      <Image
        source={sheet}
        fadeDuration={0}
        resizeMode="stretch"
        style={{ width: width * FRAMES, height: width * 2, transform: [{ translateX: -frame * width }] }}
      />
    </View>
  );
}
