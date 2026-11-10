# Wii assets

Reference: [Wii Menu community file](https://www.figma.com/design/TGmVIfpPewFNYOoNIylzfw/Wii-Menu--Community---Copy-).

- Player 1 hand: node `25:106`, original exported SVG including its shadow.
- Button reflection: node `16:10` within `16:9`.
- Footer lines: node `25:57`.
- Channel frame: node `16:17`.
- Navigation arrows: nodes `20:179` and `20:180`. The left node's canvas reflection is applied at its draw call because the SVG export contains its unreflected geometry.
- Red Player 2, green Player 3 and gold Player 4 are editable SVG derivatives of the original hand. Only the player numeral and blue color stops were changed; the outline, dimensions and shadow are preserved.
- WAV files are copied from the project's user-provided `WiiSounds` folder. Hover, click, opening and successful-response cues are embedded in the executable. Hover cues do not interrupt an active cue. Playback uses Windows WinMM; other platforms keep the visual theme without sound.

All assets are bundled. The application makes no Figma requests at runtime.
