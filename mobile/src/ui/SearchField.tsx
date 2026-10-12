// The search of a list of sessions, with the choice of its order beside it:
// grouped as the computer has them, or the newest first.
import { TextInput, View } from 'react-native';

import type { SessionOrder } from '@/store/app';
import { fonts, metrics } from '@/theme/tokens';
import { useTheme } from '@/theme/useTheme';

import { Chip } from './kit';

export function SearchField({
  query,
  onQuery,
  placeholder,
  order,
  onOrder,
  grouped,
}: {
  query: string;
  onQuery(text: string): void;
  placeholder: string;
  order: SessionOrder;
  onOrder(order: SessionOrder): void;
  /** What the grouped order is called here: by project, by worktree. */
  grouped: string;
}) {
  const t = useTheme();
  return (
    <View style={{ paddingHorizontal: 12, paddingTop: 10, gap: 8 }}>
      <TextInput
        accessibilityLabel={placeholder}
        value={query}
        onChangeText={onQuery}
        placeholder={placeholder}
        placeholderTextColor={t.textFaint}
        selectionColor={t.signal}
        cursorColor={t.signal}
        autoCapitalize="none"
        autoCorrect={false}
        clearButtonMode="while-editing"
        returnKeyType="search"
        style={{
          minHeight: metrics.touch,
          paddingHorizontal: 12,
          borderWidth: 1,
          borderColor: query ? t.signal : t.border,
          borderRadius: metrics.radius,
          backgroundColor: t.surface,
          color: t.text,
          fontFamily: fonts.mono,
          fontSize: 13,
        }}
      />
      <View accessibilityRole="radiogroup" accessibilityLabel="Order" style={{ flexDirection: 'row', gap: 8 }}>
        <Chip title={grouped} selected={order === 'grouped'} onPress={() => onOrder('grouped')} />
        <Chip
          title="Recent"
          selected={order === 'recent'}
          onPress={() => onOrder('recent')}
          hint="Lists every session, the newest first."
        />
      </View>
    </View>
  );
}
