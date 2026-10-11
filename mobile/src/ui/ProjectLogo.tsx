// A project's logo as the desktop draws it: the image scaled to fit inside a
// hairline with the control radius, or the folder glyph when there is none.
import Feather from '@expo/vector-icons/Feather';
import { useEffect, useState } from 'react';
import { Image, View } from 'react-native';
import { SvgXml } from 'react-native-svg';

import { logoKey, useLogos, wantLogo } from '@/store/logos';
import { ensureLink, useRuntime } from '@/store/runtime';
import { metrics } from '@/theme/tokens';
import { useTheme } from '@/theme/useTheme';

export function ProjectLogo({ hostId, root, size = 28 }: { hostId: string; root: string; size?: number }) {
  const t = useTheme();
  const logo = useLogos((s) => s.logos[logoKey(hostId, root)]);
  const epoch = useLogos((s) => s.epoch);
  const online = useRuntime((s) => s.conn[hostId]?.kind === 'online');
  // An image that cannot be drawn (a damaged file, an avatar that is gone)
  // gives way to the glyph instead of an empty tile.
  const [broken, setBroken] = useState<string | null>(null);

  useEffect(() => {
    wantLogo(hostId, root, (host, path) => (online ? ensureLink(host)?.projectLogo(path) : undefined));
  }, [hostId, root, online, epoch]);

  const id = logo?.kind === 'image' || logo?.kind === 'svg' ? logo.hash : logo?.kind === 'avatar' ? logo.url : null;
  const inner = size - 2;
  let image = null;
  if (logo && id !== broken) {
    if (logo.kind === 'image') {
      image = (
        <Image
          accessibilityIgnoresInvertColors
          source={{ uri: `data:image/${logo.format};base64,${logo.base64}` }}
          resizeMode="contain"
          onError={() => setBroken(logo.hash)}
          style={{ width: inner, height: inner }}
        />
      );
    } else if (logo.kind === 'svg') {
      image = (
        <SvgXml
          xml={logo.xml}
          width={inner}
          height={inner}
          onError={() => setBroken(logo.hash)}
          fallback={<Feather name="folder" size={size - 8} color={t.textMuted} />}
        />
      );
    } else if (logo.kind === 'avatar') {
      image = (
        <Image
          accessibilityIgnoresInvertColors
          source={{ uri: logo.url }}
          resizeMode="contain"
          onError={() => setBroken(logo.url)}
          style={{ width: inner, height: inner }}
        />
      );
    }
  }

  if (!image) {
    return (
      <View
        accessibilityElementsHidden
        importantForAccessibility="no-hide-descendants"
        style={{ width: size, height: size, alignItems: 'center', justifyContent: 'center' }}>
        <Feather name="folder" size={size - 8} color={t.textMuted} />
      </View>
    );
  }
  return (
    <View
      accessibilityElementsHidden
      importantForAccessibility="no-hide-descendants"
      style={{
        width: size,
        height: size,
        alignItems: 'center',
        justifyContent: 'center',
        overflow: 'hidden',
        borderRadius: metrics.radius,
        borderWidth: 1,
        borderColor: t.border,
      }}>
      {image}
    </View>
  );
}
