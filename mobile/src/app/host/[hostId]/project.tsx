// One project of a computer: its git worktrees, named by their branch as its
// Leon names them, with the sessions that ran in each. The second level of
// the chats.
import { FlashList } from '@shopify/flash-list';
import { router, useFocusEffect, useLocalSearchParams } from 'expo-router';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { RefreshControl, View } from 'react-native';

import { buildChatTree, projectRows, searchWords, type ChatRow, type Place } from '@/chat/list';
import { useApp } from '@/store/app';
import { ensureLink, refreshShared, useRuntime, watchHost } from '@/store/runtime';
import { useTheme } from '@/theme/useTheme';
import { ConnectionBanner } from '@/ui/ConnectionBanner';
import { Button, Label, Rule, Screen, Txt } from '@/ui/kit';
import { ProjectLogo } from '@/ui/ProjectLogo';
import { SearchField } from '@/ui/SearchField';
import { SessionCell } from '@/ui/SessionCell';

type Item =
  | { type: 'place'; place: Place }
  | { type: 'row'; row: ChatRow; where?: string }
  | { type: 'more'; hidden: number };

export default function ProjectChats() {
  const { hostId, project } = useLocalSearchParams<{ hostId: string; project: string }>();
  const t = useTheme();
  const host = useApp((s) => s.hosts.find((h) => h.hostId === hostId));
  const chats = useApp((s) => s.chats);
  const shared = useRuntime((s) => s.shared[hostId]);
  const worktrees = useRuntime((s) => s.worktrees[hostId]);
  const [refreshing, setRefreshing] = useState(false);

  // While this list is in front, it is read again every little while.
  useFocusEffect(useCallback(() => watchHost(hostId), [hostId]));
  // Worktrees nothing ran in are many in a busy repository: folded until asked for.
  const [showEmpty, setShowEmpty] = useState(false);

  useEffect(() => {
    ensureLink(hostId);
  }, [hostId]);

  const node = useMemo(
    () =>
      buildChatTree(
        hostId,
        Object.values(chats),
        shared?.sessions ?? [],
        shared?.projects ?? [],
        worktrees,
      ).projects.find((p) => p.key === project),
    [hostId, project, chats, shared, worktrees],
  );

  // A project with many sessions is searched by what they are called.
  const [query, setQuery] = useState('');
  const words = useMemo(() => searchWords(query), [query]);
  const order = useApp((s) => s.sessionOrder);
  const setOrder = useApp((s) => s.setSessionOrder);

  const items = useMemo<Item[]>(() => {
    if (!node) return [];
    if (order === 'recent') {
      // One list, the newest first, each session saying its worktree.
      return projectRows(node, words).map(({ row, where }): Item => ({ type: 'row', row, where }));
    }
    if (words.length > 0) {
      // Searching: only the places that hold a match, and only the matches.
      return node.places.flatMap((place): Item[] => {
        const found = place.rows.filter((row) => {
          const text = `${row.title} ${row.agent} ${place.label ?? ''}`.toLowerCase();
          return words.every((word) => text.includes(word));
        });
        return found.length === 0
          ? []
          : [{ type: 'place', place }, ...found.map((row): Item => ({ type: 'row', row }))];
      });
    }
    // The main worktree always shows: it is where a new chat most often goes.
    const shown = node.places.filter((place) => showEmpty || place.rows.length > 0 || place.main);
    const list = shown.flatMap((place): Item[] => [
      { type: 'place', place },
      ...place.rows.map((row): Item => ({ type: 'row', row })),
    ]);
    const hidden = node.places.length - shown.length;
    if (hidden > 0 || showEmpty) list.push({ type: 'more', hidden });
    return list;
  }, [node, showEmpty, words, order]);
  const anyEmpty = node?.places.some((place) => place.rows.length === 0 && !place.main) ?? false;

  if (!host) {
    return (
      <Screen title="Not found">
        <View style={{ padding: 16 }}>
          <Txt tone="muted">This computer is no longer connected to this device.</Txt>
        </View>
      </Screen>
    );
  }

  return (
    <Screen kind="Project" title={node?.title ?? 'Project'}>
      <ConnectionBanner hostId={hostId} detail={host.hostName} />
      {/* The project itself: its logo and the folder it is rooted in. */}
      {node ? (
        <View style={{ flexDirection: 'row', alignItems: 'center', gap: 12, paddingHorizontal: 16, paddingTop: 12, paddingBottom: 4 }}>
          <ProjectLogo hostId={hostId} root={node.path} size={36} />
          <View style={{ flex: 1, gap: 2 }}>
            <Label tone="faint">
              {node.count} {node.count === 1 ? 'session' : 'sessions'}
              {node.trees > 1 ? ` · ${node.trees} worktrees` : ''}
            </Label>
            <Txt variant="mono" tone="muted" numberOfLines={1} ellipsizeMode="head" style={{ fontSize: 11, lineHeight: 15 }}>
              {node.path}
            </Txt>
          </View>
        </View>
      ) : null}
      {node && node.count > 0 ? (
        <SearchField
          query={query}
          onQuery={setQuery}
          placeholder={`Search ${node.count} ${node.count === 1 ? 'session' : 'sessions'}`}
          order={order}
          onOrder={setOrder}
          grouped="By worktree"
        />
      ) : null}
      <FlashList
        data={items}
        keyboardShouldPersistTaps="handled"
        keyboardDismissMode="on-drag"
        keyExtractor={(item) =>
          item.type === 'place' ? `p:${item.place.key}` : item.type === 'row' ? item.row.key : '[more]'
        }
        getItemType={(item) => item.type}
        refreshControl={
          <RefreshControl
            refreshing={refreshing}
            tintColor={t.textMuted}
            onRefresh={() => {
              setRefreshing(true);
              void refreshShared(hostId).finally(() => setRefreshing(false));
            }}
          />
        }
        contentContainerStyle={{ paddingBottom: 16 }}
        ListEmptyComponent={
          <View style={{ padding: 16 }}>
            <Txt tone="muted">
              {words.length > 0
                ? 'No session of this project is called that.'
                : node
                  ? 'No sessions here yet. Start one below.'
                  : 'That computer no longer lists this project.'}
            </Txt>
          </View>
        }
        renderItem={({ item }) => {
          if (item.type === 'row') return <SessionCell hostId={hostId} row={item.row} where={item.where} />;
          if (item.type === 'more') {
            return anyEmpty ? (
              <View style={{ paddingHorizontal: 12, paddingTop: 14, alignItems: 'flex-start' }}>
                <Button
                  compact
                  title={showEmpty ? 'Hide empty worktrees' : `Show ${item.hidden} empty ${item.hidden === 1 ? 'worktree' : 'worktrees'}`}
                  icon={showEmpty ? 'chevron-up' : 'chevron-down'}
                  onPress={() => setShowEmpty((on) => !on)}
                />
              </View>
            ) : null;
          }
          const { place } = item;
          const name = place.label ?? 'the project folder';
          // A caption with one control at its end: text on the background
          // is never pressable, the outlined button is.
          return (
            <View
              style={{
                flexDirection: 'row',
                alignItems: 'center',
                gap: 12,
                paddingLeft: 16,
                paddingRight: 12,
                paddingTop: 14,
                paddingBottom: 6,
              }}>
              <View style={{ flex: 1, gap: 2 }}>
                <View style={{ flexDirection: 'row', alignItems: 'center', gap: 8 }}>
                  {place.label ? (
                    <Txt variant="mono" numberOfLines={1} style={{ flexShrink: 1, fontSize: 13, lineHeight: 18 }}>
                      {place.label}
                    </Txt>
                  ) : (
                    <Label tone="muted">[ Project folder ]</Label>
                  )}
                  {place.main ? <Label tone="faint">Main</Label> : null}
                  {place.rows.length === 0 ? <Label tone="faint">· empty</Label> : null}
                </View>
                <Txt variant="mono" tone="faint" numberOfLines={1} ellipsizeMode="head" style={{ fontSize: 10, lineHeight: 14, opacity: 0.7 }}>
                  {place.path}
                </Txt>
              </View>
              <Button
                compact
                title="New"
                icon="plus"
                accessibilityHint={`Starts a chat in ${name}.`}
                onPress={() =>
                  router.push({ pathname: '/host/[hostId]/new', params: { hostId, folder: place.path } })
                }
              />
            </View>
          );
        }}
      />
      <Rule />
      <View style={{ padding: 16 }}>
        <Button
          kind="primary"
          title="New chat here"
          icon="plus"
          onPress={() =>
            router.push({
              pathname: '/host/[hostId]/new',
              params: node ? { hostId, folder: node.path } : { hostId },
            })
          }
        />
      </View>
    </Screen>
  );
}
