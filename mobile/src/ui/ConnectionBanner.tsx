// The status line of a computer's screens, under the header as the desktop's
// `[ STATUS ]` footer: a light and a word while all is well, and the reason
// with a way to retry when it is not reachable.
import { View } from 'react-native';

import { connectionWord, describeConnection } from '@/link/errors';
import { retry, useRuntime } from '@/store/runtime';
import { metrics } from '@/theme/tokens';
import { useTheme } from '@/theme/useTheme';

import { Button, Label, Led, Rule, Txt } from './kit';

export function connectionColor(
  t: ReturnType<typeof useTheme>,
  kind: 'connecting' | 'online' | 'offline' | 'closed' | undefined,
): string {
  if (kind === 'online') return t.success;
  if (kind === 'offline') return t.warning;
  return t.textFaint;
}

export function ConnectionBanner({
  hostId,
  detail,
  detailLive = false,
  detailWaits = false,
}: {
  hostId: string;
  detail?: string;
  /** The detail speaks of something happening now: it gets a light. */
  detailLive?: boolean;
  /** The detail says somebody is waited for: it is said as a warning. */
  detailWaits?: boolean;
}) {
  const t = useTheme();
  const state = useRuntime((s) => s.conn[hostId]);
  const line = describeConnection(state);
  const waits = state?.kind === 'offline';
  const color = connectionColor(t, state?.kind);
  return (
    <View accessibilityLiveRegion="polite" accessibilityRole={line ? 'alert' : undefined}>
      <View style={{ backgroundColor: t.surface }}>
        <View
          style={{
            minHeight: metrics.statusLine,
            paddingHorizontal: metrics.pad,
            flexDirection: 'row',
            alignItems: 'center',
            gap: 8,
          }}>
          <Label tone="muted">[ Status ]</Label>
          <Led color={color} hollow={state?.kind !== 'online'} />
          <Label style={{ color }}>
            {waits ? '! ' : ''}
            {connectionWord(state)}
          </Label>
          {detail ? (
            <View style={{ flex: 1, flexDirection: 'row', alignItems: 'center', justifyContent: 'flex-end', gap: 6 }}>
              {detailWaits ? <Led color={t.warning} /> : detailLive ? <Led color={t.signal} /> : null}
              <Label
                numberOfLines={1}
                ellipsizeMode="middle"
                tone={detailWaits ? 'warning' : detailLive ? 'signal' : undefined}
                style={
                  detailWaits
                    ? { flexShrink: 1, textAlign: 'right' }
                    : { flexShrink: 1, textAlign: 'right', textTransform: 'none', letterSpacing: 0 }
                }>
                {detail}
              </Label>
            </View>
          ) : null}
        </View>
        {line && state?.kind !== 'connecting' && state !== undefined ? (
          <View style={{ paddingHorizontal: metrics.pad, paddingBottom: 10, gap: 8 }}>
            <Txt variant="small" tone="muted">
              {line}
            </Txt>
            {waits ? (
              <Button compact title="Try now" icon="refresh-cw" onPress={() => retry(hostId)} />
            ) : null}
          </View>
        ) : null}
      </View>
      <Rule />
    </View>
  );
}
