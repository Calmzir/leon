// Pairing: scan, paste or type the code the other computer shows.
import * as Clipboard from 'expo-clipboard';
import { router, useLocalSearchParams } from 'expo-router';
import { useMemo, useState } from 'react';
import { Alert, ScrollView, View } from 'react-native';
import { KeyboardAvoidingView } from 'react-native-keyboard-controller';

import { explain, type Explanation } from '@/link/errors';
import { isLoopback, parsePairing } from '@/link/pairing';
import type { CodeCheck } from '@/link/types';
import { tap } from '@/store/alerts';
import { useApp } from '@/store/app';
import { ensureLink, forgetHost, link, staleTwins } from '@/store/runtime';
import { Button, Callout, Field, Label, Panel, Screen, Section, Txt } from '@/ui/kit';
import { Mark } from '@/ui/Mark';

function codeHint(check: CodeCheck): { text: string; bad: boolean } {
  switch (check.kind) {
    case 'empty':
      return { text: 'Ten letters and digits, like ABCD-EFG-HJK.', bad: false };
    case 'incomplete':
      return { text: `${check.need - check.have} more to go.`, bad: false };
    case 'complete':
      return { text: 'That looks like a code.', bad: false };
    case 'bad-character':
      return {
        text: `“${check.character}” is not used in codes: they never contain 0, 1, I or O.`,
        bad: true,
      };
    case 'too-long':
      return { text: 'That is longer than a code.', bad: true };
  }
}

export default function Pair() {
  const savedRelay = useApp((s) => s.relayUrl);
  const deviceName = useApp((s) => s.deviceName);
  const [code, setCode] = useState('');
  const [relayUrl, setRelayUrl] = useState(savedRelay);
  const [name, setName] = useState(deviceName);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<Explanation | null>(null);
  // Opened from a link (the camera read the computer's QR) or back from the
  // scanner: the code and its relay come filled in. Connecting stays a tap:
  // any page can open a link.
  const given = useLocalSearchParams<{ code?: string; relay?: string }>();
  const arrived = parsePairing(given.code ?? '');
  const [note, setNote] = useState<string | null>(null);

  function take(read: { code: string; relay?: string }) {
    setCode(read.code);
    if (read.relay) setRelayUrl(read.relay);
    setFailure(null);
    setNote(null);
    tap();
  }

  // What arrived is taken once, while rendering: a later scan arrives as new
  // parameters of this same screen.
  const givenKey = `${given.code ?? ''}\n${given.relay ?? ''}`;
  const [takenKey, setTakenKey] = useState('');
  if (givenKey !== takenKey) {
    setTakenKey(givenKey);
    if (arrived) {
      setCode(arrived.code);
      if (given.relay?.trim()) setRelayUrl(given.relay.trim());
      setFailure(null);
      setNote(null);
    }
  }

  async function paste() {
    const read = parsePairing(await Clipboard.getStringAsync().catch(() => ''));
    if (read) take(read);
    else setNote('Nothing to paste: copy the code on the computer first.');
  }

  const check = useMemo(() => link().checkCode(code), [code]);
  const hint = codeHint(check);
  const deviceId = useMemo(() => {
    try {
      return link().deviceId();
    } catch (error) {
      return explain(error).headline;
    }
  }, []);

  async function pair() {
    const relay = relayUrl.trim() || savedRelay;
    const called = name.trim() || deviceName;
    setFailure(null);
    setBusy(true);
    try {
      if (called !== deviceName) {
        useApp.getState().setDeviceName(called);
        link().setDeviceName(called);
      }
      const host = await link().pair(relay, code);
      useApp.getState().addHost(host);
      // The same computer under an older identity: it can never connect
      // again, but only the person knows that it is the same one.
      const twins = staleTwins(host);
      if (twins.length > 0) {
        Alert.alert(
          `Replace the old ${host.hostName}?`,
          `This device already lists a ${host.hostName} on that relay that is not reachable. If it is this same computer, paired again, the old entry can go: the chats this device kept for it are removed with it. Nothing is deleted on the computer.`,
          [
            { text: 'Keep both', style: 'cancel' },
            { text: 'Remove the old one', style: 'destructive', onPress: () => twins.forEach(forgetHost) },
          ],
        );
      }
      ensureLink(host.hostId);
      router.replace({ pathname: '/host/[hostId]', params: { hostId: host.hostId } });
    } catch (error) {
      setFailure(explain(error, relay));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Screen kind="Pair" title="Connect a computer">
      <KeyboardAvoidingView behavior="padding" style={{ flex: 1 }}>
        <ScrollView
          keyboardShouldPersistTaps="handled"
          contentContainerStyle={{ padding: 16, gap: 18 }}>
          <View style={{ flexDirection: 'row', alignItems: 'center', gap: 14 }}>
            <Mark size={44} />
            <Txt tone="muted" style={{ flex: 1 }}>
              On the computer, open Leon and choose Share this machine. Scan the code it shows,
              or paste or type it here.
            </Txt>
          </View>

          <View style={{ flexDirection: 'row', gap: 10 }}>
            <Button
              title="Scan"
              icon="maximize"
              disabled={busy}
              style={{ flex: 1 }}
              onPress={() => router.push('/scan')}
              accessibilityHint="Opens the camera to read the code on the computer's screen"
            />
            <Button
              title="Paste"
              icon="clipboard"
              disabled={busy}
              style={{ flex: 1 }}
              onPress={() => void paste()}
            />
          </View>
          {note ? (
            <Txt variant="small" tone="faint" accessibilityLiveRegion="polite">
              {note}
            </Txt>
          ) : null}

          <View style={{ gap: 6 }}>
            <Field
              label="Pairing code"
              mono
              value={code}
              onChangeText={setCode}
              placeholder="ABCD-EFG-HJK"
              autoCapitalize="characters"
              autoComplete="off"
              autoFocus={!arrived}
              editable={!busy}
              error={hint.bad}
              returnKeyType="go"
              onSubmitEditing={() => check.kind === 'complete' && void pair()}
              style={{ fontSize: 24, letterSpacing: 3, minHeight: 64 }}
            />
            <Txt variant="small" tone={hint.bad ? 'error' : 'faint'} accessibilityLiveRegion="polite">
              {hint.text}
            </Txt>
          </View>

          {isLoopback(relayUrl) ? (
            <Callout tone="warning">
              <Txt variant="small" tone="muted">
                That computer uses a relay on itself ({relayUrl.trim()}), which a phone cannot
                reach. Point both at a relay on your network or the internet.
              </Txt>
            </Callout>
          ) : null}

          {failure ? (
            <Callout tone="error">
              <Txt variant="strong">{failure.headline}</Txt>
              <Txt variant="small" tone="muted">
                {failure.advice}
              </Txt>
            </Callout>
          ) : null}

          {busy ? (
            <Callout tone="info">
              <Txt variant="small" tone="muted">
                Waiting for the other computer. If it asks, allow “{name.trim() || deviceName}”
                after checking that it shows {deviceId}.
              </Txt>
            </Callout>
          ) : null}

          <Button
            kind="primary"
            title="Connect"
            busy={busy}
            disabled={check.kind !== 'complete'}
            onPress={() => void pair()}
          />

          <Section title="This device" style={{ paddingHorizontal: 0, paddingBottom: 0 }} />
          <Panel style={{ gap: 6 }}>
            <Label>Fingerprint</Label>
            <Txt variant="mono" selectable style={{ fontSize: 15, lineHeight: 22 }}>
              {deviceId}
            </Txt>
            <Txt variant="small" tone="faint">
              The other computer shows this next to the name below before it lets this device in.
              Compare them.
            </Txt>
          </Panel>
          <Field
            label="Name of this device"
            value={name}
            onChangeText={setName}
            editable={!busy}
            maxLength={64}
          />
          <View style={{ gap: 6 }}>
            <Field
              label="Relay server"
              mono
              value={relayUrl}
              onChangeText={setRelayUrl}
              editable={!busy}
              autoCapitalize="none"
              keyboardType="url"
              placeholder={savedRelay}
            />
            <Txt variant="small" tone="faint">
              Both devices must use the same relay. For a relay on your own network, type its
              address, like ws://192.168.1.20:8787.
            </Txt>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </Screen>
  );
}
