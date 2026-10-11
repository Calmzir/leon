// The scanner: reads the QR that "Share this machine" shows on the computer
// and goes back to pairing with the code and its relay filled in.
import { CameraView, useCameraPermissions } from 'expo-camera';
import { router } from 'expo-router';
import { useRef, useState } from 'react';
import { Linking, StyleSheet, View } from 'react-native';

import { parsePairing } from '@/link/pairing';
import { useTheme } from '@/theme/useTheme';
import { Button, Label, Screen, Txt } from '@/ui/kit';

/** The side of the frame the code is aimed at. */
const FRAME = 248;

export default function Scan() {
  const t = useTheme();
  const [permission, ask] = useCameraPermissions();
  const [other, setOther] = useState(false);
  // The camera reports the same code many times a second: the first is taken.
  const taken = useRef(false);

  function read(data: string) {
    if (taken.current) return;
    const found = parsePairing(data);
    if (!found) {
      setOther(true);
      return;
    }
    taken.current = true;
    router.dismissTo({ pathname: '/pair', params: { code: found.code, relay: found.relay ?? '' } });
  }

  const corner = (style: object) => (
    <View style={[styles.corner, { borderColor: t.signal }, style]} />
  );

  return (
    <Screen kind="Pair" title="Scan the code">
      {permission?.granted ? (
        <View style={{ flex: 1 }}>
          <CameraView
            style={StyleSheet.absoluteFill}
            facing="back"
            barcodeScannerSettings={{ barcodeTypes: ['qr'] }}
            onBarcodeScanned={({ data }) => read(data)}
          />
          <View style={styles.aim} pointerEvents="none">
            <View style={{ width: FRAME, height: FRAME }}>
              {corner({ top: 0, left: 0, borderTopWidth: 2, borderLeftWidth: 2 })}
              {corner({ top: 0, right: 0, borderTopWidth: 2, borderRightWidth: 2 })}
              {corner({ bottom: 0, left: 0, borderBottomWidth: 2, borderLeftWidth: 2 })}
              {corner({ bottom: 0, right: 0, borderBottomWidth: 2, borderRightWidth: 2 })}
            </View>
          </View>
          <View style={[styles.caption, { backgroundColor: t.background, borderTopColor: t.border }]}>
            <Label tone="muted">[ Share this machine ]</Label>
            <Txt variant="small" tone={other ? 'warning' : 'muted'} accessibilityLiveRegion="polite">
              {other
                ? 'That is not a Leon pairing code. Aim at the one under Share this machine.'
                : 'On the computer, open Leon and choose Share this machine. Aim at the code it shows.'}
            </Txt>
          </View>
        </View>
      ) : (
        <View style={{ padding: 16, gap: 14 }}>
          <Txt tone="muted">
            The camera is only used here, to read the code on the computer&apos;s screen. Nothing
            is recorded or sent.
          </Txt>
          {permission && !permission.canAskAgain ? (
            <Button title="Open the settings" onPress={() => void Linking.openSettings()} />
          ) : (
            <Button kind="primary" title="Allow the camera" onPress={() => void ask()} />
          )}
          <Button title="Type the code instead" onPress={() => router.back()} />
        </View>
      )}
    </Screen>
  );
}

const styles = StyleSheet.create({
  aim: { ...StyleSheet.absoluteFill, alignItems: 'center', justifyContent: 'center' },
  corner: { position: 'absolute', width: 28, height: 28 },
  caption: {
    position: 'absolute',
    left: 0,
    right: 0,
    bottom: 0,
    padding: 16,
    gap: 6,
    borderTopWidth: StyleSheet.hairlineWidth,
  },
});
