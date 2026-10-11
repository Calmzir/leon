// The root: fonts, the keyboard provider, the navigation stack, and the two
// moments the connections are woken (start, and coming back to the front).
import { Inter_400Regular } from '@expo-google-fonts/inter/400Regular';
import { Inter_500Medium } from '@expo-google-fonts/inter/500Medium';
import { Inter_600SemiBold } from '@expo-google-fonts/inter/600SemiBold';
import { JetBrainsMono_400Regular } from '@expo-google-fonts/jetbrains-mono/400Regular';
import { JetBrainsMono_600SemiBold } from '@expo-google-fonts/jetbrains-mono/600SemiBold';
import { useFonts } from 'expo-font';
import { Stack } from 'expo-router';
import * as SplashScreen from 'expo-splash-screen';
import { StatusBar } from 'expo-status-bar';
import * as SystemUI from 'expo-system-ui';
import { useEffect } from 'react';
import { AppState } from 'react-native';
import { KeyboardProvider } from 'react-native-keyboard-controller';
import { SafeAreaProvider } from 'react-native-safe-area-context';

import { keepHearing } from '@/store/background';
import { connectAll, wake } from '@/store/runtime';
import { useIsDark, useMotion, useTheme } from '@/theme/useTheme';
import { Toast } from '@/ui/Toast';

void SplashScreen.preventAutoHideAsync();

export default function RootLayout() {
  const t = useTheme();
  const isDark = useIsDark();
  const motion = useMotion();
  const [loaded, failed] = useFonts({
    Inter_400Regular,
    Inter_500Medium,
    Inter_600SemiBold,
    JetBrainsMono_400Regular,
    JetBrainsMono_600SemiBold,
  });
  const ready = loaded || failed !== null;

  useEffect(() => {
    void SystemUI.setBackgroundColorAsync(t.background);
  }, [t.background]);

  useEffect(() => {
    if (!ready) return;
    void SplashScreen.hideAsync();
    connectAll();
    const subscription = AppState.addEventListener('change', (state) => {
      if (state === 'active') wake();
    });
    const hearing = keepHearing();
    return () => {
      subscription.remove();
      hearing();
    };
  }, [ready]);

  if (!ready) return null;

  return (
    <SafeAreaProvider>
      <KeyboardProvider>
        <StatusBar style={isDark ? 'light' : 'dark'} />
        <Stack
          screenOptions={{
            headerShown: false,
            contentStyle: { backgroundColor: t.background },
            animation: motion.open === 0 ? 'none' : 'default',
          }}
        />
        <Toast />
      </KeyboardProvider>
    </SafeAreaProvider>
  );
}
