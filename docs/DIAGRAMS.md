# Panels and page maps

Popover is a resizable panel with a pointer on any of its four sides. The pointer has five positions. Its natural size is 222×187; Auto-size restores both dimensions. It supports fill color, rich text and text styling. Multiple text lines align left; a single line is centered. Dark fills use light text. Very small panels retain their fill while the pointer and outline disappear at their respective size thresholds.

Site map lays out a hierarchy from text. Begin with one root, then use leading dashes or tabs for child levels. The diagram computes its size from its labels and can run vertically or horizontally. Each node can link to a page, a web address or the previous presentation page. Populate from project replaces the text and page links together, so Undo restores both. Empty text produces one empty node. Invalid hierarchies display a readable message and keep saved destinations available while the text is repaired.

Both controls share layout and drawing between the editor and PNG/SVG exports. Page maps retain rich text, explicit line breaks, per-node link regions and component-local property overrides. Original geometry uses deterministic curves and triangulated panel fills.

The feature checkpoint is accepted on macOS: native editing and history, full-process restart/resave, and PNG/SVG export checks pass. Automated tests cover geometry, local component edits, invalid-text recovery and saved tiny panels. Presentation link activation and native component editing have not received a dedicated native acceptance pass. Original typography and rasterization still have known fidelity limits. Native Windows and Linux acceptance remains open.
