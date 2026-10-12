// How the turns of a chat are run: the agent's own permission modes, its
// models and how hard it should think. The lists are what the agent on that
// computer says it offers; "Default" leaves a choice to that computer.
import { useMemo, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  Modal,
  Pressable,
  ScrollView,
  TextInput,
  View,
  useWindowDimensions,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

import { effortsFor } from '@/chat/model';
import type { AgentOptions, TurnOptions } from '@/link/types';
import { fonts } from '@/theme/tokens';
import { useMotion, useTheme } from '@/theme/useTheme';

import { Button, Chip, Label, Led, Rule, Txt } from './kit';

type Entry = { id: string | undefined; label: string; detail?: string; warn?: boolean };

/** More models than this are not listed whole: they are searched. */
const MANY_MODELS = 12;
/** How many of them are listed under a search. */
const SHOWN_MODELS = 30;

function Choice({
  entry,
  selected,
  onPress,
}: {
  entry: Entry;
  selected: boolean;
  onPress(): void;
}) {
  const t = useTheme();
  return (
    <Pressable
      accessibilityRole="radio"
      accessibilityState={{ selected }}
      accessibilityLabel={entry.detail ? `${entry.label}. ${entry.detail}` : entry.label}
      onPress={onPress}
      style={({ pressed }) => ({
        flexDirection: 'row',
        gap: 12,
        minHeight: 48,
        paddingLeft: 14,
        paddingRight: 16,
        paddingVertical: 11,
        borderBottomWidth: 1,
        borderBottomColor: t.border,
        borderLeftWidth: 2,
        borderLeftColor: selected ? t.signal : 'transparent',
        backgroundColor: pressed || selected ? t.surface2 : 'transparent',
      })}>
      <View style={{ paddingTop: 5 }}>
        <Led color={selected ? t.signal : t.textFaint} hollow={!selected} />
      </View>
      <View style={{ flex: 1, gap: 3 }}>
        <Label tone={selected ? 'signal' : entry.warn ? 'warning' : 'text'}>
          {entry.warn ? '! ' : ''}
          {entry.label}
        </Label>
        {entry.detail ? (
          <Txt variant="small" tone={entry.warn ? 'warning' : 'muted'}>
            {entry.detail}
          </Txt>
        ) : null}
      </View>
    </Pressable>
  );
}

export function OptionsSheet({
  visible,
  agentName,
  offered,
  value,
  running = false,
  onChange,
  onClose,
}: {
  visible: boolean;
  agentName: string;
  /** What the agent offers over there; nothing while it is being asked. */
  offered: AgentOptions | undefined;
  value: TurnOptions;
  /** A turn is in flight: say what a change does to it. */
  running?: boolean;
  onChange(next: TurnOptions): void;
  onClose(): void;
}) {
  const t = useTheme();
  const motion = useMotion();
  const insets = useSafeAreaInsets();
  const { height } = useWindowDimensions();

  const modes: Entry[] = [
    { id: undefined, label: 'Default', detail: 'The mode that computer is set up for.' },
    ...(offered?.modes ?? []).map((m) => ({
      id: m.id,
      label: m.label,
      detail: m.description,
      warn: m.dangerous,
    })),
  ];
  const models: Entry[] = [
    { id: undefined, label: 'Default', detail: "That computer's own choice." },
    ...(offered?.models ?? []).map((m) => ({ id: m.id, label: m.label, detail: m.description })),
  ];
  const efforts = effortsFor(offered, value.model);

  // An agent of many providers lists more models than a sheet can hold:
  // they are searched by name, the chosen one always in sight.
  const [query, setQuery] = useState('');
  const many = models.length > MANY_MODELS;
  const searching = many && query.trim().length > 0;
  const { listed, more } = useMemo(() => {
    if (!many) return { listed: models, more: 0 };
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    const found = models.filter(
      (entry) =>
        entry.id !== undefined &&
        entry.id !== value.model &&
        words.every((word) => `${entry.label} ${entry.id}`.toLowerCase().includes(word)),
    );
    const pinned = models.filter((entry) => entry.id === undefined || entry.id === value.model);
    return {
      listed: [...pinned, ...found.slice(0, SHOWN_MODELS)],
      more: Math.max(0, found.length - SHOWN_MODELS),
    };
    // `models` is rebuilt on every render from `offered`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [many, query, value.model, offered]);

  function pickMode(entry: Entry) {
    const apply = () => onChange({ ...value, mode: entry.id });
    if (!entry.warn || entry.id === value.mode) {
      apply();
      return;
    }
    Alert.alert(
      `Use ${entry.label.toLowerCase()}?`,
      'The agent may then run any command and change any file on that computer, as you, without asking. There is no sandbox.',
      [
        { text: 'Cancel', style: 'cancel' },
        { text: 'Use it', style: 'destructive', onPress: apply },
      ],
    );
  }

  function pickModel(entry: Entry) {
    const next = effortsFor(offered, entry.id);
    onChange({
      ...value,
      model: entry.id,
      effort: value.effort && next.includes(value.effort) ? value.effort : undefined,
    });
  }

  return (
    <Modal
      visible={visible}
      transparent
      animationType={motion.open === 0 ? 'none' : 'fade'}
      onRequestClose={onClose}>
      <Pressable
        accessibilityLabel="Close"
        accessibilityRole="button"
        onPress={onClose}
        style={{ flex: 1, backgroundColor: t.scrim }}
      />
      <View
        accessibilityViewIsModal
        style={{
          maxHeight: height * 0.86,
          backgroundColor: t.surface,
          borderTopWidth: 1,
          borderTopColor: t.signal,
          paddingBottom: insets.bottom + 12,
        }}>
        <ScrollView style={{ flexShrink: 1 }} keyboardShouldPersistTaps="handled">
          {/* While a model is searched for, the modes step aside: the
              keyboard leaves room for little else. */}
          {searching ? null : (
            <>
          <View style={{ padding: 16, gap: 4 }}>
            <Label tone="muted">[ Mode ]</Label>
            <Txt variant="small" tone="faint">
              What {agentName} may do without asking. Whatever it has to ask, it asks you here.
              {offered && !offered.followUps
                ? ' It cannot be told more while it works: wait for its answer, or stop it.'
                : ''}
            </Txt>
          </View>
          <Rule />
          {!offered ? (
            <View style={{ padding: 16, flexDirection: 'row', alignItems: 'center', gap: 10 }}>
              <ActivityIndicator color={t.textMuted} />
              <Txt variant="small" tone="muted">
                Asking {agentName} on that computer what it offers…
              </Txt>
            </View>
          ) : null}
          <View accessibilityRole="radiogroup">
            {modes.map((entry) => (
              <Choice
                key={entry.id ?? ''}
                entry={entry}
                selected={entry.id === value.mode}
                onPress={() => pickMode(entry)}
              />
            ))}
          </View>
            </>
          )}
          {models.length > 1 ? (
            <>
              <View style={{ paddingHorizontal: 16, paddingTop: 18, paddingBottom: 8 }}>
                <Label tone="muted">[ Model ]</Label>
              </View>
              {many ? (
                <View style={{ paddingHorizontal: 16, paddingBottom: 10 }}>
                  <TextInput
                    accessibilityLabel="Search models"
                    value={query}
                    onChangeText={setQuery}
                    placeholder={`Search ${models.length - 1} models`}
                    placeholderTextColor={t.textFaint}
                    autoCapitalize="none"
                    autoCorrect={false}
                    style={{
                      minHeight: 44,
                      paddingHorizontal: 12,
                      borderWidth: 1,
                      borderColor: t.border,
                      borderRadius: 6,
                      backgroundColor: t.background,
                      color: t.text,
                      fontFamily: fonts.mono,
                      fontSize: 13,
                    }}
                  />
                </View>
              ) : null}
              <Rule />
              <View accessibilityRole="radiogroup">
                {listed.map((entry) => (
                  <Choice
                    key={entry.id ?? ''}
                    entry={entry}
                    selected={entry.id === value.model}
                    onPress={() => pickModel(entry)}
                  />
                ))}
              </View>
              {more > 0 ? (
                <View style={{ paddingHorizontal: 16, paddingTop: 10 }}>
                  <Txt variant="small" tone="faint">
                    {more} more. Type part of a name to find one.
                  </Txt>
                </View>
              ) : null}
            </>
          ) : null}
          {efforts.length > 0 ? (
            <View style={{ paddingHorizontal: 16, paddingTop: 18, gap: 10 }}>
              <Label tone="muted">[ Effort ]</Label>
              <View style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
                <Chip
                  title="Default"
                  selected={!value.effort}
                  onPress={() => onChange({ ...value, effort: undefined })}
                />
                {efforts.map((effort) => (
                  <Chip
                    key={effort}
                    title={effort}
                    selected={value.effort === effort}
                    onPress={() => onChange({ ...value, effort })}
                  />
                ))}
              </View>
            </View>
          ) : null}
          <View style={{ padding: 16, gap: 6 }}>
            {running ? (
              <Txt variant="small" tone="faint">
                {offered?.liveChanges
                  ? 'The mode and the model change for the turn in flight too; the effort from the next one.'
                  : 'A change applies from the next message.'}
              </Txt>
            ) : null}
            {offered?.assumed ? (
              <Txt variant="small" tone="faint">
                {agentName} could not be asked what it offers there: this is the usual list.
              </Txt>
            ) : offered?.version ? (
              <Txt variant="small" tone="faint">
                As offered by {agentName} {offered.version} on that computer.
              </Txt>
            ) : null}
          </View>
        </ScrollView>
        <View style={{ paddingHorizontal: 16, paddingTop: 8 }}>
          <Button title="Done" onPress={onClose} />
        </View>
      </View>
    </Modal>
  );
}
