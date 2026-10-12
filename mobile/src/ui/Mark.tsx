// Leon's mark, from brand/logo/final/mark.svg. It is one flat shape, so it
// ships as a mask the theme tints with its `logo` colour: crisp at every size
// used here without an SVG library, and its cuts show the surface behind it.
import { Image, View, type StyleProp, type ViewStyle } from 'react-native';

import { useTheme } from '@/theme/useTheme';

const MASK = require('../../assets/mark-mask.png');

export function Mark({ size = 56, color }: { size?: number; color?: string }) {
  const t = useTheme();
  return (
    <Image
      accessibilityLabel="Leon"
      accessibilityRole="image"
      source={MASK}
      style={{ width: size, height: size, tintColor: color ?? t.logo }}
    />
  );
}

/** The mark, large and barely there, behind an empty state as on the desktop. */
export function Watermark({ size = 300, style }: { size?: number; style?: StyleProp<ViewStyle> }) {
  const t = useTheme();
  return (
    <View
      pointerEvents="none"
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      style={[{ position: 'absolute', right: -size * 0.22, bottom: size * 0.12, opacity: 0.06 }, style]}>
      <Image source={MASK} style={{ width: size, height: size, tintColor: t.logo }} />
    </View>
  );
}
