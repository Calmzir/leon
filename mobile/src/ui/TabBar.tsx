// The bar at the bottom: mono labels under a glyph, the place in front in the
// accent with a rule over it. The Den says how many agents wait for the
// person, wherever they are looked at from.
import Feather from '@expo/vector-icons/Feather';
import { Tabs } from 'expo-router';
import { useMemo, type ComponentProps } from 'react';
import { Pressable, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

import { lionsOf, tally } from '@/chat/den';
import { tap } from '@/store/alerts';
import { useApp } from '@/store/app';
import { useRuntime } from '@/store/runtime';
import { useTheme } from '@/theme/useTheme';

import { Label } from './kit';

/** What the navigator hands its bar. */
type BarProps = Parameters<NonNullable<ComponentProps<typeof Tabs>['tabBar']>>[0];

const ICONS: Record<string, 'home' | 'grid' | 'settings'> = {
  index: 'home',
  den: 'grid',
  settings: 'settings',
};

export function TabBar({ state, descriptors, navigation }: BarProps) {
  const t = useTheme();
  const insets = useSafeAreaInsets();
  const hosts = useApp((s) => s.hosts);
  const dens = useRuntime((s) => s.dens);
  const waiting = useMemo(() => tally(lionsOf(hosts, dens)).waiting, [hosts, dens]);
  return (
    <View
      accessibilityRole="tablist"
      style={{
        flexDirection: 'row',
        backgroundColor: t.background,
        borderTopWidth: 1,
        borderTopColor: t.border,
        paddingBottom: insets.bottom,
      }}>
      {state.routes.map((route, at) => {
        const focused = state.index === at;
        const title = descriptors[route.key]?.options.title ?? route.name;
        const color = focused ? t.signal : t.textMuted;
        const badge = route.name === 'den' && waiting > 0 ? waiting : 0;
        return (
          <Pressable
            key={route.key}
            accessibilityRole="tab"
            accessibilityLabel={badge ? `${title}, ${badge} waiting for you` : title}
            accessibilityState={{ selected: focused }}
            onPress={() => {
              const event = navigation.emit({ type: 'tabPress', target: route.key, canPreventDefault: true });
              if (!focused && !event.defaultPrevented) {
                tap();
                navigation.navigate(route.name, route.params);
              }
            }}
            style={({ pressed }) => ({
              flex: 1,
              alignItems: 'center',
              gap: 5,
              paddingTop: 9,
              paddingBottom: 9,
              borderTopWidth: 2,
              borderTopColor: focused ? t.signal : 'transparent',
              marginTop: -1,
              backgroundColor: pressed ? t.surface2 : 'transparent',
            })}>
            <View>
              <Feather name={ICONS[route.name] ?? 'circle'} size={20} color={color} />
              {badge ? (
                <View
                  style={{
                    position: 'absolute',
                    top: -5,
                    left: 14,
                    minWidth: 16,
                    height: 16,
                    paddingHorizontal: 4,
                    alignItems: 'center',
                    justifyContent: 'center',
                    backgroundColor: t.warning,
                  }}>
                  <Label style={{ color: t.background, fontSize: 10, lineHeight: 14 }}>{badge}</Label>
                </View>
              ) : null}
            </View>
            <Label style={{ color }}>{title}</Label>
          </Pressable>
        );
      })}
    </View>
  );
}
