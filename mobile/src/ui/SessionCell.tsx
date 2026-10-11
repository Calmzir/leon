// One session in a list: a raised cell with the agent's colour on its edge.
// The things to press are the only ones that sit on a surface.
import Feather from '@expo/vector-icons/Feather';
import { router } from 'expo-router';
import { Pressable, View } from 'react-native';

import type { ChatRow } from '@/chat/list';
import { useApp } from '@/store/app';
import { agentColor } from '@/theme/tokens';
import { useTheme } from '@/theme/useTheme';

import { useClock } from './clock';
import { Label, Led, Txt } from './kit';

/** How lately a session must have changed to count as being worked on. */
const MOVING_MS = 90_000;

/** How long ago, in the fewest words. */
export function ago(at: number): string {
  const minutes = Math.max(0, Math.round((Date.now() - at) / 60000));
  if (minutes < 1) return 'now';
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h`;
  return `${Math.round(hours / 24)} d`;
}

/** Opens a row: a shared session gets a record on this device the first time. */
export function openRow(hostId: string, row: ChatRow) {
  let chatId = row.chatId;
  if (!chatId && row.session) {
    const { session } = row;
    chatId = row.key;
    if (!useApp.getState().chats[chatId]) {
      useApp.getState().putChat({
        id: chatId,
        hostId,
        agent: session.agent,
        cwd: session.cwd || undefined,
        title: session.title || 'Untitled',
        externalId: session.externalId,
        resumeId: session.canResume ? session.externalId : undefined,
        sessionIds: [session.externalId],
        createdAt: session.startedMs,
        updatedAt: session.updatedMs,
      });
    }
  }
  if (chatId) router.push({ pathname: '/chat/[chatId]', params: { chatId } });
}

export function SessionCell({
  hostId,
  row,
  where,
}: {
  hostId: string;
  row: ChatRow;
  /** Where it runs, said on a list that mixes computers and projects. */
  where?: string;
}) {
  const t = useTheme();
  const now = useClock();
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={`${row.title}, ${row.agent}${row.waiting ? ', waiting for your answer' : row.running ? ', answering' : ''}`}
      accessibilityHint={row.canContinue ? 'Opens the chat.' : 'Opens it to read; it cannot be continued from here.'}
      onPress={() => openRow(hostId, row)}
      style={({ pressed }) => ({
        flexDirection: 'row',
        alignItems: 'center',
        gap: 12,
        minHeight: 60,
        marginHorizontal: 12,
        marginBottom: 6,
        paddingVertical: 12,
        paddingLeft: 12,
        paddingRight: 10,
        backgroundColor: pressed ? t.surface2 : t.surface,
        borderWidth: 1,
        borderColor: pressed ? t.elevatedBorder : t.border,
        borderLeftWidth: 2,
        borderLeftColor: agentColor(t, row.agent),
      })}>
      <View style={{ flex: 1, gap: 4 }}>
        <Txt numberOfLines={2}>{row.title}</Txt>
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: 6 }}>
          <Led color={agentColor(t, row.agent)} />
          <Label style={{ color: agentColor(t, row.agent) }}>{row.agent}</Label>
          <Label numberOfLines={1} style={{ flex: 1, textTransform: where ? 'none' : 'uppercase' }}>
            {where ? `· ${where}` : row.session ? `· ${row.session.messages} msg` : ''}
            {row.canContinue ? '' : ' · read only'}
          </Label>
        </View>
      </View>
      {row.waiting ? (
        // Its agent asked something: it does nothing until it is answered.
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: 6 }}>
          <Led color={t.warning} />
          <Label tone="warning" accessibilityLabel="Waiting for your answer">
            Waiting
          </Label>
        </View>
      ) : row.running ? (
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: 6 }}>
          <Led color={t.signal} />
          <Label tone="signal" accessibilityLabel="Answering">
            Live
          </Label>
        </View>
      ) : row.session && now - row.updatedAt < MOVING_MS ? (
        // It changed on that computer a moment ago: an agent is at it there.
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: 6 }}>
          <Led color={agentColor(t, row.agent)} />
          <Label style={{ color: agentColor(t, row.agent) }} accessibilityLabel="Changed just now">
            Active
          </Label>
        </View>
      ) : (
        <Label>{ago(row.updatedAt)}</Label>
      )}
      <Feather name="chevron-right" size={18} color={t.textMuted} />
    </Pressable>
  );
}
