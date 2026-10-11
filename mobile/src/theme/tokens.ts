// The Leon theme: colours, type, shape and motion from brand/tokens.md.
// Sizes follow the desktop's scale, raised for a phone held at arm's length
// (body 13 px there, 16 pt here); the ratios and the roles are the same.

export type Palette = {
  background: string;
  surface: string;
  surface2: string;
  border: string;
  gridMark: string;
  guide: string;
  scrim: string;
  elevatedBorder: string;
  text: string;
  textMuted: string;
  textFaint: string;
  /** The accent as text or a thin mark. */
  signal: string;
  /** The fill of the one primary button of a screen. */
  accentFill: string;
  onAccentFill: string;
  success: string;
  warning: string;
  error: string;
  info: string;
  /** The mark: the accent on dark, ink on light. */
  logo: string;
  agentClaude: string;
  agentCodex: string;
  agentOpencode: string;
};

export const dark: Palette = {
  background: '#0A0A0A',
  surface: '#141414',
  surface2: '#1C1C1C',
  border: '#262626',
  gridMark: '#555555',
  guide: '#333333',
  scrim: 'rgba(10, 10, 10, 0.64)',
  elevatedBorder: '#5A5A5A',
  text: '#FAFAF9',
  textMuted: '#A8A29E',
  textFaint: '#8C8580',
  signal: '#FFEA00',
  accentFill: '#FFEA00',
  onAccentFill: '#0A0A0A',
  success: '#4DF688',
  warning: '#FF9500',
  error: '#FF5E5E',
  info: '#6EFAFF',
  logo: '#FFEA00',
  agentClaude: '#D97757',
  agentCodex: '#FAFAF9',
  agentOpencode: '#F1ECEC',
};

export const light: Palette = {
  background: '#FAFAF9',
  surface: '#FFFFFF',
  surface2: '#F5F5F4',
  border: '#E7E5E4',
  gridMark: '#AAA8A7',
  guide: '#D3D1D0',
  scrim: 'rgba(12, 10, 9, 0.46)',
  elevatedBorder: '#A8A29E',
  text: '#0C0A09',
  textMuted: '#57534E',
  textFaint: '#78716C',
  signal: '#756600',
  accentFill: '#FFEA00',
  onAccentFill: '#0A0A0A',
  success: '#047857',
  warning: '#A06000',
  error: '#B91C1C',
  info: '#0E7490',
  logo: '#0A0A0A',
  agentClaude: '#B8583A',
  agentCodex: '#0C0A09',
  agentOpencode: '#211E1E',
};

export const fonts = {
  sans: 'Inter_400Regular',
  sansMedium: 'Inter_500Medium',
  sansSemibold: 'Inter_600SemiBold',
  mono: 'JetBrainsMono_400Regular',
  monoSemibold: 'JetBrainsMono_600SemiBold',
} as const;

export const metrics = {
  /** Controls, buttons, chips, inputs. */
  radius: 6,
  /** Panels, rows and cells are square. */
  radiusCell: 0,
  hairline: 1,
  /** Smallest thing a finger is asked to hit. */
  touch: 44,
  pad: 16,
  gap: 12,
  textBody: 15,
  textSmall: 13,
  textLabel: 11,
  textTitle: 22,
  textCode: 13,
  /** The header and the status line share their heights with the desktop. */
  header: 48,
  statusLine: 32,
  /** Arms of a corner mark. */
  tick: 8,
} as const;

export const motion = {
  /** Press and focus. */
  quick: 120,
  /** Something that opens. */
  open: 160,
} as const;

/** The colour that stands for an agent, by its catalogue tag. */
export function agentColor(palette: Palette, agent: string): string {
  if (agent === 'claude') return palette.agentClaude;
  if (agent === 'codex') return palette.agentCodex;
  if (agent === 'opencode') return palette.agentOpencode;
  return palette.textMuted;
}
