# ON/OFF switch

New switches start on and measure 42 × 24 in both styles. The library preview and insertion share this default. Size is computed; geometry fields cannot resize it. Stored dimensions remain intact when loading older files, while layout uses the intrinsic size. The control has no caption editor.

The inspector exposes State, Color and Link. The on color defaults to RGB 106, 168, 79 and accepts custom colors. Off retains the custom color for switching on again; its displayed colors are light gray in sketch style and white with a gray border in clean style. The white thumb is right when on and left when off. Original parametric contours provide the drawing in canvas, SVG, PNG and PDF through the common scene pipeline. Whole-control links support page, back and web destinations.

Document version 12 changes the sparse default to on. Reading versions 1–11 preserves the old off default with a legacy checked value when no explicit state exists. Explicit state and checked values retain their meaning. Migration visits groups, alternates, retained pages, component definitions and library content. Keeping the checked representation preserves instance checked overrides. Reset on a standalone switch uses the new default; reset in an instance inherits its source.

Named files and recovery records cross the same migration boundary. A previously clean recovery baseline remains clean; unsaved changes remain unsaved. Reading does not rewrite either file.

Focused checks cover both styles, default/custom on/off color and thumb placement, insertion/library preview, fixed geometry, history, link export, round trips and older document/recovery migration. Native visual comparison remains part of G01 acceptance; these checks do not claim exact contour identity or native interaction acceptance.
