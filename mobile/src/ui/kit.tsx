// The building blocks every screen uses, in the desktop's vocabulary: mono
// `[ SECTION ]` labels, square panels with 1 px rules and corner marks, 6 px
// outlined controls in mono capitals, square status lights, and one accent
// button per screen.
import Feather from '@expo/vector-icons/Feather';
import Constants from 'expo-constants';
import { router } from 'expo-router';
import { useState, type ComponentProps, type ReactNode } from 'react';
import {
  ActivityIndicator,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  View,
  type StyleProp,
  type TextInputProps,
  type TextProps,
  type TextStyle,
  type ViewStyle,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

import { fonts, metrics } from '@/theme/tokens';
import { useTheme } from '@/theme/useTheme';

import { Mark } from './Mark';

export type IconName = ComponentProps<typeof Feather>['name'];

type Tone = 'text' | 'muted' | 'faint' | 'signal' | 'error' | 'warning' | 'success';

function useTone(tone: Tone): string {
  const t = useTheme();
  switch (tone) {
    case 'text':
      return t.text;
    case 'muted':
      return t.textMuted;
    case 'faint':
      return t.textFaint;
    case 'signal':
      return t.signal;
    case 'error':
      return t.error;
    case 'warning':
      return t.warning;
    case 'success':
      return t.success;
  }
}

type TxtProps = TextProps & {
  variant?: 'body' | 'small' | 'title' | 'strong' | 'mono';
  tone?: Tone;
};

/** Text in the brand's type. Sizes follow the system's text size setting. */
export function Txt({ variant = 'body', tone = 'text', style, ...rest }: TxtProps) {
  const color = useTone(tone);
  return (
    <Text
      maxFontSizeMultiplier={variant === 'title' ? 1.4 : 2}
      {...rest}
      style={[styles[variant], { color }, style]}
    />
  );
}

/** A small uppercase mono label: field names, states, metadata. */
export function Label({ tone = 'faint', style, ...rest }: TextProps & { tone?: Tone }) {
  const color = useTone(tone);
  return <Text maxFontSizeMultiplier={1.5} {...rest} style={[styles.label, { color }, style]} />;
}

/** The name of a section, in the desktop's `[ NAME ]` form. */
export function Section({
  title,
  right,
  style,
}: {
  title: string;
  right?: ReactNode;
  style?: StyleProp<ViewStyle>;
}) {
  return (
    <View style={[styles.section, style]}>
      <Label tone="muted" accessibilityRole="header" accessibilityLabel={title}>
        [ {title} ]
      </Label>
      {right}
    </View>
  );
}

/** A status light: a small square of one state colour. */
export function Led({ color, hollow = false }: { color: string; hollow?: boolean }) {
  return (
    <View
      style={[
        styles.led,
        { borderColor: color, backgroundColor: hollow ? 'transparent' : color },
      ]}
    />
  );
}

type ButtonProps = {
  title: string;
  onPress(): void;
  /** `primary` is the accent fill: at most one per screen. */
  kind?: 'primary' | 'outline' | 'danger';
  icon?: IconName;
  busy?: boolean;
  disabled?: boolean;
  /** A control inside a row or a line rather than a screen's own action. */
  compact?: boolean;
  style?: StyleProp<ViewStyle>;
  accessibilityHint?: string;
};

export function Button({
  title,
  onPress,
  kind = 'outline',
  icon,
  busy = false,
  disabled = false,
  compact = false,
  style,
  accessibilityHint,
}: ButtonProps) {
  const t = useTheme();
  const off = disabled || busy;
  // Off, a button keeps its shape and loses its colour: never a faded accent.
  const ink = disabled
    ? t.textFaint
    : kind === 'primary'
      ? t.onAccentFill
      : kind === 'danger'
        ? t.error
        : t.text;
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={title}
      accessibilityHint={accessibilityHint}
      accessibilityState={{ disabled: off, busy }}
      disabled={off}
      onPress={onPress}
      style={({ pressed }) => [
        styles.button,
        compact && styles.buttonCompact,
        disabled
          ? { backgroundColor: t.surface2, borderColor: t.border }
          : kind === 'primary'
            ? { backgroundColor: t.accentFill, borderColor: t.accentFill, opacity: pressed ? 0.85 : 1 }
            : {
                backgroundColor: pressed ? t.surface2 : 'transparent',
                borderColor: kind === 'danger' ? t.error : t.elevatedBorder,
              },
        style,
      ]}>
      {busy ? (
        <ActivityIndicator color={ink} />
      ) : (
        <>
          {icon ? <Feather name={icon} size={15} color={ink} /> : null}
          <Text maxFontSizeMultiplier={1.5} style={[styles.buttonText, { color: ink }]}>
            {title}
          </Text>
        </>
      )}
    </Pressable>
  );
}

/** A control that is only an icon. Always labelled. */
export function IconButton({
  icon,
  label,
  onPress,
  disabled = false,
  color,
}: {
  icon: IconName;
  label: string;
  onPress(): void;
  disabled?: boolean;
  color?: string;
}) {
  const t = useTheme();
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={{ disabled }}
      disabled={disabled}
      hitSlop={4}
      onPress={onPress}
      style={({ pressed }) => [
        styles.iconButton,
        { backgroundColor: pressed ? t.surface2 : 'transparent' },
      ]}>
      <Feather name={icon} size={20} color={disabled ? t.textFaint : (color ?? t.textMuted)} />
    </Pressable>
  );
}

type FieldProps = TextInputProps & { label: string; mono?: boolean; error?: boolean };

export function Field({ label, mono = false, error = false, style, onFocus, onBlur, ...rest }: FieldProps) {
  const t = useTheme();
  const [focused, setFocused] = useState(false);
  return (
    <View style={styles.fieldWrap}>
      <Label>{label}</Label>
      <TextInput
        accessibilityLabel={label}
        placeholderTextColor={t.textFaint}
        selectionColor={t.signal}
        cursorColor={t.signal}
        autoCorrect={false}
        {...rest}
        onFocus={(event) => {
          setFocused(true);
          onFocus?.(event);
        }}
        onBlur={(event) => {
          setFocused(false);
          onBlur?.(event);
        }}
        style={[
          styles.field,
          {
            color: t.text,
            backgroundColor: t.surface,
            borderColor: error ? t.error : focused ? t.signal : t.border,
            fontFamily: mono ? fonts.mono : fonts.sans,
          },
          style,
        ]}
      />
    </View>
  );
}

export function Rule({ style }: { style?: StyleProp<ViewStyle> }) {
  const t = useTheme();
  return <View style={[{ height: metrics.hairline, backgroundColor: t.border }, style]} />;
}

/** The L-shaped marks at the corners of a panel, as on the desktop's cards. */
function CornerTicks({ color }: { color: string }) {
  const arm = metrics.tick;
  const corner = (style: ViewStyle) => (
    <View pointerEvents="none" style={[{ position: 'absolute', width: arm, height: arm, borderColor: color }, style]} />
  );
  return (
    <>
      {corner({ top: -1, left: -1, borderTopWidth: 1, borderLeftWidth: 1 })}
      {corner({ top: -1, right: -1, borderTopWidth: 1, borderRightWidth: 1 })}
      {corner({ bottom: -1, left: -1, borderBottomWidth: 1, borderLeftWidth: 1 })}
      {corner({ bottom: -1, right: -1, borderBottomWidth: 1, borderRightWidth: 1 })}
    </>
  );
}

/** A square panel: a hairline all round and a mark at each corner. */
export function Panel({
  children,
  style,
  accent = false,
}: {
  children: ReactNode;
  style?: StyleProp<ViewStyle>;
  /** Outlined in the accent: the one state of a screen that matters most. */
  accent?: boolean;
}) {
  const t = useTheme();
  return (
    <View
      style={[
        styles.panel,
        { borderColor: accent ? t.signal : t.border, backgroundColor: t.surface },
        style,
      ]}>
      {children}
      <CornerTicks color={accent ? t.signal : t.gridMark} />
    </View>
  );
}

/** "Leon <version> · by Zavu", where the desktop signs its empty window. */
export function Signature({ style }: { style?: StyleProp<TextStyle> }) {
  const version = Constants.expoConfig?.version;
  return (
    <Label style={[{ textAlign: 'center', textTransform: 'none', letterSpacing: 0 }, style]}>
      Leon{version ? ` ${version}` : ''} · by Zavu
    </Label>
  );
}

/** The page: brand background, safe areas, and a header with a rule under it. */
export function Screen({
  title,
  kind,
  subtitle,
  back = true,
  brand = false,
  right,
  children,
  bottomInset = true,
}: {
  title: string;
  /** What the title names, shown before it as `[ KIND ]`. */
  kind?: string;
  subtitle?: string;
  back?: boolean;
  /** The home header: the mark and the wordmark instead of a title. */
  brand?: boolean;
  right?: ReactNode;
  children: ReactNode;
  /** Off when the screen pads its own bottom (a composer above the keyboard). */
  bottomInset?: boolean;
}) {
  const t = useTheme();
  const insets = useSafeAreaInsets();
  return (
    <View
      style={{
        flex: 1,
        backgroundColor: t.background,
        paddingTop: insets.top,
        paddingBottom: bottomInset ? insets.bottom : 0,
        paddingLeft: insets.left,
        paddingRight: insets.right,
      }}>
      <View style={styles.header}>
        {back ? (
          <IconButton icon="chevron-left" label="Back" onPress={() => router.back()} color={t.text} />
        ) : (
          <View style={{ width: metrics.pad - 4 }} />
        )}
        {brand ? (
          <View accessibilityRole="header" accessibilityLabel={title} style={styles.brand}>
            <Mark size={24} />
            <Text maxFontSizeMultiplier={1.3} style={[styles.wordmark, { color: t.text }]}>
              Leon
            </Text>
          </View>
        ) : (
          <View style={styles.headerTitles}>
            <View style={styles.headerLine}>
              {kind ? <Label tone="muted">[ {kind} ]</Label> : null}
              <Text
                accessibilityRole="header"
                numberOfLines={1}
                maxFontSizeMultiplier={1.3}
                style={[styles.headerTitle, { color: t.text }]}>
                {title}
              </Text>
            </View>
            {subtitle ? (
              <Text
                numberOfLines={1}
                ellipsizeMode="head"
                maxFontSizeMultiplier={1.3}
                style={[styles.headerSubtitle, { color: t.textFaint }]}>
                {subtitle}
              </Text>
            ) : null}
          </View>
        )}
        {right ?? <View style={{ width: metrics.pad - 4 }} />}
      </View>
      <Rule />
      {children}
    </View>
  );
}

/** A full-width row of a list: square, with a rule under it. */
export function Row({
  onPress,
  onLongPress,
  children,
  accessibilityLabel,
  accessibilityHint,
  disabled = false,
  selected = false,
}: {
  onPress?(): void;
  onLongPress?(): void;
  children: ReactNode;
  accessibilityLabel: string;
  accessibilityHint?: string;
  disabled?: boolean;
  /** The open row: on `surface_2`, with the accent at its edge. */
  selected?: boolean;
}) {
  const t = useTheme();
  return (
    <Pressable
      accessibilityRole={onPress ? 'button' : 'text'}
      accessibilityLabel={accessibilityLabel}
      accessibilityHint={accessibilityHint}
      accessibilityState={selected ? { selected } : undefined}
      disabled={disabled || !onPress}
      onPress={onPress}
      onLongPress={onLongPress}
      style={({ pressed }) => [
        styles.row,
        {
          borderBottomColor: t.border,
          borderLeftColor: selected ? t.signal : 'transparent',
          backgroundColor: pressed || selected ? t.surface2 : 'transparent',
        },
      ]}>
      {children}
    </Pressable>
  );
}

/** A state as a square light and a word: never colour alone. */
export function Status({ color, word }: { color: string; word: string }) {
  return (
    <View style={styles.status}>
      <Led color={color} />
      <Label style={{ color }}>{word}</Label>
    </View>
  );
}

/** A choice among a few: outlined mono chips, the chosen one in the accent. */
export function Chip({
  title,
  selected,
  onPress,
  disabled = false,
  hint,
  mark,
}: {
  title: string;
  selected: boolean;
  onPress(): void;
  disabled?: boolean;
  hint?: string;
  /** A colour that stands for the choice (an agent's), as a square before it. */
  mark?: string;
}) {
  const t = useTheme();
  return (
    <Pressable
      accessibilityRole="radio"
      accessibilityLabel={title}
      accessibilityHint={hint}
      accessibilityState={{ selected, disabled }}
      disabled={disabled}
      onPress={onPress}
      style={({ pressed }) => [
        styles.chip,
        {
          borderColor: disabled ? t.border : selected ? t.signal : t.elevatedBorder,
          backgroundColor: pressed || selected ? t.surface2 : 'transparent',
        },
      ]}>
      {mark ? <Led color={disabled ? t.textFaint : mark} hollow={!selected} /> : null}
      <Text
        maxFontSizeMultiplier={1.5}
        style={[
          styles.buttonText,
          { color: disabled ? t.textFaint : selected ? t.signal : t.text },
        ]}>
        {title}
      </Text>
    </Pressable>
  );
}

/** A note in the flow of a screen: a rule on the left in the tone's colour. */
export function Callout({
  tone,
  children,
  style,
}: {
  tone: 'info' | 'warning' | 'error';
  children: ReactNode;
  style?: StyleProp<ViewStyle>;
}) {
  const t = useTheme();
  const color = tone === 'error' ? t.error : tone === 'warning' ? t.warning : t.gridMark;
  const icon: IconName =
    tone === 'error' ? 'x-octagon' : tone === 'warning' ? 'alert-triangle' : 'info';
  return (
    <View
      style={[
        styles.callout,
        { borderColor: t.border, borderLeftColor: color, backgroundColor: t.surface },
        style,
      ]}>
      <Feather name={icon} size={14} color={color} style={{ marginTop: 3 }} />
      <View style={{ flex: 1, gap: 6 }}>{children}</View>
    </View>
  );
}

const text = (family: string, size: number, lineHeight: number): TextStyle => ({
  fontFamily: family,
  fontSize: size,
  lineHeight,
});

const styles = StyleSheet.create({
  body: text(fonts.sans, metrics.textBody, 22),
  small: text(fonts.sans, metrics.textSmall, 19),
  strong: text(fonts.sansSemibold, metrics.textBody, 22),
  title: { ...text(fonts.sansSemibold, metrics.textTitle, 28), letterSpacing: -0.44 },
  mono: text(fonts.mono, 12, 18),
  label: {
    fontFamily: fonts.monoSemibold,
    fontSize: metrics.textLabel,
    lineHeight: 16,
    letterSpacing: 0.5,
    textTransform: 'uppercase',
  },
  section: {
    minHeight: 40,
    paddingHorizontal: metrics.pad,
    paddingTop: 18,
    paddingBottom: 8,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: metrics.gap,
  },
  led: { width: 7, height: 7, borderWidth: 1 },
  button: {
    minHeight: 48,
    paddingHorizontal: 16,
    paddingVertical: 10,
    borderRadius: metrics.radius,
    borderWidth: metrics.hairline,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
  },
  buttonCompact: { minHeight: metrics.touch, paddingHorizontal: 12, alignSelf: 'flex-start' },
  buttonText: {
    fontFamily: fonts.monoSemibold,
    fontSize: 12,
    lineHeight: 16,
    letterSpacing: 0.6,
    textTransform: 'uppercase',
  },
  iconButton: {
    width: metrics.touch,
    height: metrics.touch,
    borderRadius: metrics.radius,
    alignItems: 'center',
    justifyContent: 'center',
  },
  fieldWrap: { gap: 6 },
  field: {
    minHeight: 48,
    paddingHorizontal: 12,
    paddingVertical: 10,
    borderRadius: metrics.radius,
    borderWidth: metrics.hairline,
    fontSize: metrics.textBody,
  },
  panel: { borderWidth: metrics.hairline, borderRadius: metrics.radiusCell, padding: metrics.pad },
  header: {
    minHeight: metrics.header,
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 4,
  },
  brand: { flex: 1, flexDirection: 'row', alignItems: 'center', gap: 10, paddingHorizontal: 4 },
  wordmark: { fontFamily: fonts.sansSemibold, fontSize: 19, lineHeight: 24, letterSpacing: -0.66 },
  headerTitles: { flex: 1, paddingHorizontal: 4, paddingVertical: 5 },
  headerLine: { flexDirection: 'row', alignItems: 'center', gap: 8 },
  headerTitle: {
    flex: 1,
    fontFamily: fonts.sansSemibold,
    fontSize: 15,
    lineHeight: 20,
    letterSpacing: -0.3,
  },
  headerSubtitle: { fontFamily: fonts.mono, fontSize: 11, lineHeight: 15 },
  row: {
    minHeight: 56,
    paddingLeft: metrics.pad - 2,
    paddingRight: metrics.pad,
    paddingVertical: 11,
    borderBottomWidth: metrics.hairline,
    borderLeftWidth: 2,
    flexDirection: 'row',
    alignItems: 'center',
    gap: metrics.gap,
  },
  status: { flexDirection: 'row', alignItems: 'center', gap: 6 },
  chip: {
    minHeight: metrics.touch,
    paddingHorizontal: 12,
    borderRadius: metrics.radius,
    borderWidth: metrics.hairline,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    gap: 8,
  },
  callout: {
    flexDirection: 'row',
    gap: 10,
    padding: 12,
    borderWidth: metrics.hairline,
    borderLeftWidth: 2,
  },
});
