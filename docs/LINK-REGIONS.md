# Link regions and overlays

Presentation and export use the same scene link regions. A later ordinary control blocks earlier targets inside its visible bounds, including blank interior space. Ancestor crop masks restrict the covered region. A control marked as markup, or one whose first painted item is text, preserves the earlier target beneath it.

Text-first describes the actual drawing, not the library category. Labels, headings, subtitles, paragraphs, links, tag clouds, breadcrumbs, link bars and icon labels can begin with text. Text fields, text areas, field sets and placeholder text strokes have graphical backgrounds or strokes and block links. Empty or constrained controls follow the drawing they actually produce. Opacity and mask scope markers are not painted items.

The covering control's own markup flag governs this rule. A group marked as markup does not make an ordinary child stop blocking links. Keep this local flag separate from inherited markup in layout. Preserve stored flags, component resolution and normal catalog defaults.

An overlay's own link still wins within its link region. Subtract those regions from earlier controls before appending the overlay links. Leave the control's internal item-link ordering unchanged. Whole-control and inline/item targets retain their existing precedence. Linked groups continue to override descendants at their established paint-order position.

## Inspector editing

Whole-control targets now use the same Page, Go back, Web address and No link
choices as row/item targets. Controls whose catalog exposes a target use their
typed inspector. Existing page-only routes, including groups, retain their
capability through the complete picker. Each eligible single-selection inspector
shows one picker. The projected page-ID field is refreshed only when applying
another inspector edit, so it cannot overwrite a newly selected URL or Back target.

Incomplete web-address input is a UI draft until Apply; Cancel discards the draft.
Drafts are scoped to project session, surface, editing context, control and row.
An invalid target cannot enter document data. A changed saved target invalidates
its prior draft. Group/instance edits use canonical node properties and local
component overrides, preserving source definitions and Undo/Redo. Existing link
regions, occlusion and export semantics are unchanged.

The picker also offers Link to new wireframe and Link to duplicate of this
wireframe. Both create a destination and assign its links in one Undo step, keeping
the current page, version, nested context and selection. A dismissible notice
opens the new page on request. Undo removes the destination and restores all prior
links; Redo restores the same destination identity. Failure rolls back this action
without losing an earlier text edit.

The destination uses the first control's text or the selected row label as its
name, retaining literal markup. It is inserted last among the current page's live
siblings. A retained parent does not impose a hidden nesting level. Duplicating
copies the active version's complete control content before assigning the new
link. It does not copy other versions, page notes or review discussions. From a
component source editor it copies the complete owning library surface. Shared
component definitions and assets keep their identities.

Multiple selections expose common whole-control links and the union of applicable
row links. Mixed targets are explicit, and choosing an already matching target
still applies it to the other controls. Missing rows remain untouched. App bars
use their own icon-link selection, keeping each final Menu slot aligned despite
differing icon counts. Source-library roots do not expose a whole-control target.
New destinations, common target changes and row changes retain component-local
property overrides where applicable.

Inline rich-text destinations have a separate Text links inspector for supported
text controls. It lists parsed labels in source order, preserving duplicate-label
positions. Table links follow actual text cells, excluding choice markers and the
column-width footer. Setting or clearing a destination retains the bracketed label,
nested formatting, surrounding whitespace and escapes. Parentheses and commas in
web addresses are escaped in source and decoded for rendering. New destinations
retain the inline label’s formatting markers in the name (for example,
`[*Café*]` creates `*Café*`), while the inspector displays the rendered label.
Table cell comma escapes are decoded in the name. All actions share atomic history,
component-local overrides, mixed-selection and missing-position behavior with the
other pickers. Editing the source text invalidates unfinished destination drafts;
collapsing and scrolling the inspector remains independent of text changes.

Canvas and inspector text display live page destinations by name. Committing or
previewing an edit resolves exact names back to stable page IDs. Existing links
keep their identities through surrounding-text edits, label changes and identifiable
reordering, including duplicate page names. Indistinguishable duplicate occurrences
retain their positional association. New typed duplicate names choose the first live
page in document order; the destination picker can choose a specific page. Existing
URLs and missing page targets remain unchanged unless explicitly edited. Back and
explicit page target syntax remain supported. Target punctuation is escaped safely;
commas are escaped in plain text as well as table cells.

Native paragraph acceptance covers readable double-click/full-source selection,
no-op acceptance without history, typed destination resolution, rename refresh,
duplicate-name identity, inspector blur, cancelled drafts retaining Redo, exact saved
Undo/Redo and cold reopen. Previews, table/escaped-name and duplicate-label collision
cases have focused domain/application coverage. Exact popup/selection appearance,
IME and further serialized-name edge cases retain their limits. Evidence: external
readable-link-edit-current/report.md and preceding native-inline-current/report.md.

Project import collects and remaps inline page destinations alongside typed links.
Omitted and removed pages receive fresh broken-destination identities, preventing
accidental links to equal numeric IDs in the destination. Page reuse, alternates,
component sources and local overrides follow the same mapping. Table parsing uses
the underlying control kind before component paths change. Orphan overrides retain
all explicit destinations that could belong to ordinary text or table cells.
Literal group names, notes and escaped markup remain untouched. Unsafe formatting
complexity rejects the isolated transfer instead of partially rewriting it.

Foreign library previews/imports clear external inline page destinations while
preserving labels, formatting, URL and Back targets. Ordinary clipboard data carries
optional project/session identity: same-project links survive, including a first
Save or rename of the same open document. Foreign or unidentified older clipboard
payloads clear page targets rather than bind to unrelated equal-numbered pages.
Qualified older source clipboards retain their known same-project behavior.
These clipboard-only fields do not change the project-file format.

Native appearance, input and system clipboard round trips remain to be checked on
the final candidate. Direct text editing remains available.

## Verification

Before the correction, regressions demonstrate a plain label cutting a hole in a linked card and a marked note blocking the card. Focused rendering checks cover text versus framed controls, local versus parent markup, storage round trips, own-target priority, absence of duplicate cross-control export targets, and inline links over a cropped card without document or Undo changes. Existing nested group, component and list-link checks remain required. Native product/export acceptance and the completed suite results are tracked in SESSION-SPEC.md; passing region checks alone do not establish all display or platform behavior.

Native macOS acceptance covers nine overlay/background clicks and all-page PDF export from an original eight-card fixture. Native and programmatic PDFs are byte-identical, with 268800 unit-cell target checks passing. This is scoped link-policy acceptance; broader interaction and platform coverage remain open.

## Rotated inline hints

Rotated text highlights follow the existing four-corner text shape. Presentation
retains one full hint per original region. Screen-sized PDF hints are clipped to
each visible fragment so covering controls remain opaque. PDF annotation rectangles
and pointer hit regions are unchanged. Paper-sized hint arrows retain their existing
axis-aligned appearance. Diagonal bounding-box corners can still interrupt an
underlying target; paired pointer-overlap acceptance remains open.

The correction passes26 application presentation tests and44 rendering/export/link
tests. Native diagonal/card navigation and PDF export pass; native and programmatic
PDFs are byte-identical. Native transient highlight screenshots were inconclusive;
application-frame geometry is verified. External rotated-links-current/report.md
records the candidate, failures before correction, artifacts and scope.
