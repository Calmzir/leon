// The line across the top that says an agent elsewhere needs the person or is
// done. Pressing it opens the chat; it leaves by itself.
import Feather from '@expo/vector-icons/Feather';
import { Pressable, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

import { dismissToast, openChat, useAlerts } from '@/store/alerts';
import { metrics } from '@/theme/tokens';
import { useTheme } from '@/theme/useTheme';

import { IconButton, Label, Led, Txt } from './kit';

export function Toast() {
  const t = useTheme();
  const insets = useSafeAreaInsets();
  const toast = useAlerts((s) => s.toast);
  if (!toast) return null;
  const color = toast.kind === 'waiting' ? t.warning : toast.kind === 'failed' ? t.error : t.success;
  return (
    <View
      accessibilityLiveRegion="assertive"
      style={{ position: 'absolute', top: insets.top + 6, left: 10, right: 10 }}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel={`${toast.title}. ${toast.line}`}
        accessibilityHint="Opens the chat."
        onPress={() => openChat(toast.chatId)}
        style={({ pressed }) => ({
          flexDirection: 'row',
          alignItems: 'center',
          gap: 10,
          minHeight: 56,
          paddingLeft: 12,
          backgroundColor: pressed ? t.surface2 : t.surface,
          borderWidth: metrics.hairline,
          borderColor: t.elevatedBorder,
          borderLeftWidth: 2,
          borderLeftColor: color,
        })}>
        <Led color={color} />
        <View style={{ flex: 1, paddingVertical: 8, gap: 2 }}>
          <Label style={{ color }}>
            {toast.kind === 'waiting' ? '? ' : toast.kind === 'failed' ? '! ' : '✓ '}
            {toast.title}
          </Label>
          {toast.line ? (
            <Txt variant="small" tone="muted" numberOfLines={1}>
              {toast.line}
            </Txt>
          ) : null}
        </View>
        <Feather name="chevron-right" size={16} color={t.textMuted} />
        <IconButton icon="x" label="Dismiss" onPress={dismissToast} />
      </Pressable>
    </View>
  );
}
