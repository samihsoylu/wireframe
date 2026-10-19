# Horizontal rules

New horizontal rules insert at 100 × 10. The inspector offers Solid, Dashed, Dotted,
Breakline Top and Breakline Bottom, border color and border opacity. The ordinary
styles have a fixed height of 10; the two breaklines have a fixed height of 38.
Only the width can be dragged or edited. Group and component resizing retain these
heights, and local component edits leave the source unchanged.

Dots use a one-unit mark and eight-unit gap. Dashes use 12/24 mark/gap lengths in
Sketch and 6/12 in Clean, with three/two-unit strokes. Breaklines use a torn edge
from width 65 upward and a straight edge below 65. Top masks the area above the
edge with opaque white; Bottom masks below it. Border opacity affects the ink,
including solid rules, without fading that white area. Original product geometry
is shared by the canvas and PNG, SVG and PDF exports.

Format 14 preserves the requested frames, centered solid strokes and default ink
of lines saved by earlier versions. Their hidden `legacy_rule_frame` flag retains
the old freely sized frame for Solid, Dashed and Dotted. Selecting a breakline
temporarily uses height 38; selecting an ordinary style restores the stored frame.
Changing the width while a breakline is selected can explicitly replace that
stored frame through the normal resize command. Migration covers groups,
alternates, retained pages, definitions and library content. Recovery migration
retains clean/dirty status and does not rewrite the input file or recovery record.

Dashed and dotted marks use one filled capsule each. This gives both rule
orientations rounded ends in native, raster and vector output without dark seams
where translucent cap and stem pieces would overlap. Quick Add retains the
horizontal rule's sole canvas kind while suppressing duplicate legacy choices.

Integration checks cover sizing, history, component overrides, migrations, the
64/65-unit transition, complete white meshes and exports. Application checks cover
inspector visibility, Quick Add insertion and pointer-driven width changes.
Native macOS comparisons now cover insertion, all five Sketch styles, disabled
height fields, width64/65, readable choices, opacity editing, exact saved Undo/Redo
and cold reopen. The final candidate's dashed/dotted caps and half-opacity fading
were visually checked at actual size. External `native-rules-current/report.md`
pins the candidates and captures. Selection styling, inspector arrangement,
native Clean/vertical comparison, colored-background masks, physical resizing and
opacity cadence retain their specific limits; this is not complete visual parity.
