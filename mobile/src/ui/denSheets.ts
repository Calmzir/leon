// The lions of the Den in 2.5D, as the desktop draws them: for each agent's
// colour, six individuals, each a strip of ten pictures (200 by 260) of the
// same lion: standing, typing (2), reading (2), thinking (2), staring,
// asleep, out cold. Drawn by the desktop's own renderer:
//
//   cargo run -p leon-den --features den3d --example den_lions_png -- mobile/assets/den
import type { ImageSourcePropType } from 'react-native';

/** Pictures in a strip, and the shape of one. */
export const FRAMES = 10;
export const FRAME_WIDE = 200;
export const FRAME_HIGH = 260;

export const SHEETS: Record<string, ImageSourcePropType[]> = {
  claude: [
    require('../../assets/den/claude_0.png'),
    require('../../assets/den/claude_1.png'),
    require('../../assets/den/claude_2.png'),
    require('../../assets/den/claude_3.png'),
    require('../../assets/den/claude_4.png'),
    require('../../assets/den/claude_5.png'),
  ],
  codex: [
    require('../../assets/den/codex_0.png'),
    require('../../assets/den/codex_1.png'),
    require('../../assets/den/codex_2.png'),
    require('../../assets/den/codex_3.png'),
    require('../../assets/den/codex_4.png'),
    require('../../assets/den/codex_5.png'),
  ],
  opencode: [
    require('../../assets/den/opencode_0.png'),
    require('../../assets/den/opencode_1.png'),
    require('../../assets/den/opencode_2.png'),
    require('../../assets/den/opencode_3.png'),
    require('../../assets/den/opencode_4.png'),
    require('../../assets/den/opencode_5.png'),
  ],
  other: [
    require('../../assets/den/other_0.png'),
    require('../../assets/den/other_1.png'),
    require('../../assets/den/other_2.png'),
    require('../../assets/den/other_3.png'),
    require('../../assets/den/other_4.png'),
    require('../../assets/den/other_5.png'),
  ],
};
