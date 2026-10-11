// A new chat: which agent, in which folder of the other computer.
import Feather from '@expo/vector-icons/Feather';
import { router, useLocalSearchParams } from 'expo-router';
import { useCallback, useMemo, useState } from 'react';
import { ActivityIndicator, ScrollView, View } from 'react-native';

import { explain } from '@/link/errors';
import { useApp } from '@/store/app';
import { ensureLink, link, useRuntime } from '@/store/runtime';
import { agentColor } from '@/theme/tokens';
import { useTheme } from '@/theme/useTheme';
import { ConnectionBanner } from '@/ui/ConnectionBanner';
import { Button, Callout, Chip, Led, Row, Rule, Screen, Section, Txt } from '@/ui/kit';

function parentOf(path: string): string | null {
  const trimmed = path.replace(/\/+$/, '');
  const cut = trimmed.lastIndexOf('/');
  if (cut < 0) return null;
  return cut === 0 ? (trimmed.length > 1 ? '/' : null) : trimmed.slice(0, cut);
}

function join(path: string, name: string): string {
  return path.endsWith('/') ? `${path}${name}` : `${path}/${name}`;
}

export default function NewChat() {
  const { hostId, folder: startIn } = useLocalSearchParams<{ hostId: string; folder?: string }>();
  const t = useTheme();
  const agents = useMemo(() => link().agents(), []);
  const installed = useRuntime((s) => s.installed[hostId]);
  const projects = useRuntime((s) => s.shared[hostId]?.projects);
  const online = useRuntime((s) => s.conn[hostId]?.kind === 'online');

  const [picked, setAgent] = useState<string | null>(null);
  // Until one is picked, the agent last used on this computer is the one
  // offered, or the first one installed there.
  const lastUsed = useApp((s) => {
    let latest: { agent: string; updatedAt: number } | undefined;
    for (const chat of Object.values(s.chats)) {
      if (chat.hostId !== hostId || chat.sessionIds.length === 0 || chat.externalId) continue;
      if (!latest || chat.updatedAt > latest.updatedAt) latest = chat;
    }
    return latest?.agent;
  });
  const usable = (id: string | undefined) =>
    id !== undefined && agents.some((a) => a.id === id) && (installed?.includes(id) ?? false);
  const agent =
    picked ??
    (usable(lastUsed) ? lastUsed! : (agents.find((a) => installed?.includes(a.id))?.id ?? null));
  const [folder, setFolder] = useState<string | null>(startIn ?? null);
  const [browsing, setBrowsing] = useState<string | null>(null);
  const [subdirs, setSubdirs] = useState<string[] | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const browse = useCallback(
    async (path: string | null) => {
      const opened = ensureLink(hostId);
      if (!opened) return;
      setProblem(null);
      setSubdirs(null);
      try {
        const target = path ?? (await opened.homeDir());
        setBrowsing(target);
        setFolder(target);
        setSubdirs(await opened.subdirs(target));
      } catch (error) {
        const said = explain(error);
        setProblem(`${said.headline} ${said.advice}`);
        setSubdirs([]);
      }
    },
    [hostId],
  );

  function start() {
    if (!agent) return;
    const id = `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;
    const now = Date.now();
    useApp.getState().putChat({
      id,
      hostId,
      agent,
      cwd: folder ?? undefined,
      title: '',
      sessionIds: [],
      createdAt: now,
      updatedAt: now,
    });
    router.replace({ pathname: '/chat/[chatId]', params: { chatId: id } });
  }

  const noneInstalled = installed !== undefined && !agents.some((a) => installed.includes(a.id));
  const up = browsing ? parentOf(browsing) : null;

  return (
    <Screen kind="New" title="Chat">
      <ConnectionBanner hostId={hostId} />
      <ScrollView contentContainerStyle={{ paddingBottom: 16 }}>
        <Section title="Agent" />
        <View style={{ paddingHorizontal: 16, paddingBottom: 16, gap: 10 }}>
          <View accessibilityRole="radiogroup" style={{ flexDirection: 'row', flexWrap: 'wrap', gap: 8 }}>
            {agents.map((a) => {
              const missing = installed !== undefined && !installed.includes(a.id);
              return (
                <Chip
                  key={a.id}
                  title={missing ? `${a.name} (not installed)` : a.name}
                  selected={agent === a.id}
                  mark={agentColor(t, a.id)}
                  disabled={missing}
                  onPress={() => setAgent(a.id)}
                  hint={missing ? `Install ${a.program} on that computer to use it.` : undefined}
                />
              );
            })}
          </View>
          {installed === undefined && online ? (
            <Txt variant="small" tone="faint">
              Looking for the agents installed there…
            </Txt>
          ) : null}
          {noneInstalled ? (
            <Callout tone="warning">
              <Txt variant="small" tone="muted">
                None of these agents is installed on that computer. Install{' '}
                {agents.map((a) => a.program).join(' or ')} there, then come back.
              </Txt>
            </Callout>
          ) : null}
        </View>

        <Rule />

        <View style={{ gap: 4 }}>
          <Section title="Folder" />
          <View
            style={{
              marginHorizontal: 16,
              paddingHorizontal: 12,
              paddingVertical: 10,
              flexDirection: 'row',
              gap: 8,
              backgroundColor: t.surface,
              borderWidth: 1,
              borderColor: t.border,
            }}>
            <Txt variant="mono" tone="signal">
              $
            </Txt>
            <Txt variant="mono" tone={folder ? 'text' : 'faint'} selectable style={{ flex: 1 }}>
              {folder ?? 'the folder Leon runs in on that computer'}
            </Txt>
          </View>

          {projects && projects.length > 0 && browsing === null ? (
            <View style={{ marginTop: 12 }}>
              <Section title="Projects" />
              <Rule />
              {projects.map((p) => (
                <Row
                  key={p.root}
                  accessibilityLabel={`${p.name}, ${p.root}`}
                  accessibilityHint="Works in this project."
                  selected={folder === p.root}
                  onPress={() => setFolder(p.root)}>
                  <Led color={folder === p.root ? t.signal : t.textFaint} hollow={folder !== p.root} />
                  <View style={{ flex: 1 }}>
                    <Txt>{p.name}</Txt>
                    <Txt variant="mono" tone="faint" numberOfLines={1} ellipsizeMode="head" style={{ fontSize: 11, lineHeight: 15 }}>
                      {p.root}
                    </Txt>
                  </View>
                </Row>
              ))}
            </View>
          ) : null}

          {browsing === null ? (
            <View style={{ padding: 16 }}>
              <Button
                title="Browse folders"
                icon="folder"
                disabled={!online}
                onPress={() => void browse(folder)}
              />
            </View>
          ) : (
            <View style={{ marginTop: 12 }}>
              <Rule />
              {up ? (
                <Row accessibilityLabel="Up one folder" onPress={() => void browse(up)}>
                  <Feather name="corner-left-up" size={16} color={t.textMuted} />
                  <Txt variant="mono" tone="muted">
                    ..
                  </Txt>
                </Row>
              ) : null}
              {subdirs === null ? (
                <View style={{ padding: 20 }}>
                  <ActivityIndicator color={t.textMuted} />
                </View>
              ) : (
                subdirs.map((name) => (
                  <Row
                    key={name}
                    accessibilityLabel={name}
                    accessibilityHint="Opens this folder."
                    onPress={() => void browse(join(browsing, name))}>
                    <Feather name="folder" size={16} color={t.textMuted} />
                    <Txt variant="mono" style={{ flex: 1, fontSize: 13 }} numberOfLines={1} ellipsizeMode="middle">
                      {name}
                    </Txt>
                    <Feather name="chevron-right" size={16} color={t.textFaint} />
                  </Row>
                ))
              )}
              {subdirs?.length === 0 && !problem ? (
                <Txt variant="small" tone="faint" style={{ padding: 16 }}>
                  No folders inside this one.
                </Txt>
              ) : null}
              {problem ? (
                <Callout tone="error" style={{ margin: 16 }}>
                  <Txt variant="small" tone="muted">
                    {problem}
                  </Txt>
                </Callout>
              ) : null}
            </View>
          )}
        </View>
      </ScrollView>
      <Rule />
      <View style={{ padding: 16 }}>
        <Button
          kind="primary"
          title={folder && browsing ? 'Start chat in this folder' : 'Start chat'}
          disabled={!agent}
          onPress={start}
        />
      </View>
    </Screen>
  );
}
