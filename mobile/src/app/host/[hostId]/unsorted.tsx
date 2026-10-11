// The sessions of a computer that belong to no project, under the folder
// each ran in: the "unsorted" node of its Leon's sidebar.
import { FlashList } from '@shopify/flash-list';
import { router, useFocusEffect, useLocalSearchParams } from 'expo-router';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { RefreshControl, View } from 'react-native';

import { buildChatTree, type ChatRow, type FolderNode } from '@/chat/list';
import { useApp } from '@/store/app';
import { ensureLink, refreshShared, useRuntime, watchHost } from '@/store/runtime';
import { useTheme } from '@/theme/useTheme';
import { ConnectionBanner } from '@/ui/ConnectionBanner';
import { Button, Label, Rule, Screen, Txt } from '@/ui/kit';
import { SessionCell } from '@/ui/SessionCell';

type Item = { type: 'folder'; folder: FolderNode } | { type: 'row'; row: ChatRow };

export default function UnsortedChats() {
  const { hostId } = useLocalSearchParams<{ hostId: string }>();
  const t = useTheme();
  const host = useApp((s) => s.hosts.find((h) => h.hostId === hostId));
  const chats = useApp((s) => s.chats);
  const shared = useRuntime((s) => s.shared[hostId]);
  const worktrees = useRuntime((s) => s.worktrees[hostId]);
  const [refreshing, setRefreshing] = useState(false);

  // While this list is in front, it is read again every little while.
  useFocusEffect(useCallback(() => watchHost(hostId), [hostId]));

  useEffect(() => {
    ensureLink(hostId);
  }, [hostId]);

  const tree = useMemo(
    () =>
      buildChatTree(
        hostId,
        Object.values(chats),
        shared?.sessions ?? [],
        shared?.projects ?? [],
        worktrees,
      ),
    [hostId, chats, shared, worktrees],
  );

  const items = useMemo<Item[]>(
    () =>
      tree.unsorted.flatMap((folder): Item[] => [
        { type: 'folder', folder },
        ...folder.rows.map((row): Item => ({ type: 'row', row })),
      ]),
    [tree],
  );

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
    <Screen kind="Unsorted" title={host.hostName}>
      <ConnectionBanner hostId={hostId} detail={host.hostName} />
      <View style={{ paddingHorizontal: 16, paddingTop: 12, paddingBottom: 4, gap: 2 }}>
        <Label tone="faint">
          {tree.unsortedCount} {tree.unsortedCount === 1 ? 'session' : 'sessions'} · {tree.unsorted.length}{' '}
          {tree.unsorted.length === 1 ? 'folder' : 'folders'}
        </Label>
        <Txt variant="small" tone="faint">
          Sessions that ran outside every project, by folder.
        </Txt>
      </View>
      <FlashList
        data={items}
        keyExtractor={(item) => (item.type === 'folder' ? `f:${item.folder.key}` : item.row.key)}
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
            <Txt tone="muted">Nothing unsorted: every session belongs to a project.</Txt>
          </View>
        }
        renderItem={({ item }) => {
          if (item.type === 'row') return <SessionCell hostId={hostId} row={item.row} />;
          const { folder } = item;
          // A caption with one control at its end, as on a project's worktrees.
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
                <Txt variant="mono" numberOfLines={1} style={{ fontSize: 13, lineHeight: 18 }}>
                  {folder.title}
                </Txt>
                {folder.path ? (
                  <Txt variant="mono" tone="faint" numberOfLines={1} ellipsizeMode="head" style={{ fontSize: 10, lineHeight: 14, opacity: 0.7 }}>
                    {folder.path}
                  </Txt>
                ) : null}
              </View>
              <Button
                compact
                title="New"
                icon="plus"
                accessibilityHint={`Starts a chat in ${folder.title}.`}
                onPress={() =>
                  router.push({
                    pathname: '/host/[hostId]/new',
                    params: folder.path ? { hostId, folder: folder.path } : { hostId },
                  })
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
          title="New chat"
          icon="plus"
          onPress={() => router.push({ pathname: '/host/[hostId]/new', params: { hostId } })}
        />
      </View>
    </Screen>
  );
}
