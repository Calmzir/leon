// A lion of the Den: one picture of its strip, the picture moving with the
// beat of the screen. The strips are the desktop's lions in 2.5D (see
// `denSheets.ts`).
import { Image, View } from 'react-native';

import { framesOf, seedOf } from '@/chat/den';
import type { DenCub } from '@/link/types';

import { FRAME_HIGH, FRAME_WIDE, FRAMES, SHEETS } from './denSheets';

export function Lion({
  cub,
  width,
  beat,
}: {
  cub: DenCub;
  /** The width of the picture; its height follows. */
  width: number;
  /** Counts up while the den is in front; still when motion is reduced. */
  beat: number;
}) {
  const height = (width * FRAME_HIGH) / FRAME_WIDE;
  const seed = seedOf(cub);
  const sheets = SHEETS[cub.agent] ?? SHEETS.other ?? [];
  const sheet = sheets[seed % Math.max(sheets.length, 1)];
  const frames = framesOf(cub.state);
  // Lions do not move in step: each starts its round somewhere else.
  const frame = frames[(beat + seed) % frames.length] ?? 0;
  if (!sheet) return <View style={{ width, height }} />;
  return (
    <View
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      style={{ width, height, overflow: 'hidden' }}>
      <Image
        source={sheet}
        fadeDuration={0}
        resizeMode="stretch"
        style={{ width: width * FRAMES, height, transform: [{ translateX: -frame * width }] }}
      />
    </View>
  );
}
