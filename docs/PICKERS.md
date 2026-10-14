# Date and time dialogs

Date picker and Time picker are illustrative controls for mobile and desktop mockups. Both start at 320×473, retain manual dimensions, and switch from a top heading to a left heading when width reaches height. Date picker offers Month and Year views. Their date/time content is a local snapshot, not an editable form value or a functional system picker.

Month view starts on Sunday, shows only the current month's dates and highlights today. Year view shows seven years in portrait and five in landscape, with the current year enlarged. The date heading follows the project font size. Time picker shows a twelve-hour digital clock, AM/PM state and a dial whose hand points at the current hour. Midnight displays as 12:xx AM. Its selected label uses the same contrast rule at all hours, including noon and midnight.

CANCEL and OK have separate destinations under Button links. Each button disappears when there is insufficient horizontal space, while its destination remains stored. Visible OK always retains its own destination. Text and link hit regions are clipped to the control and its parent crop.

Moving a picker or editing another control retains its captured date/time. Resizing, changing drawing style or project font, reopening, and changing a date picker's Month/Year mode refresh the relevant snapshot. Time is not serialized and no timer continuously advances these illustrative controls. Digital and analog content share one captured clock value.

Canvas, thumbnails, library previews and PNG/SVG exports use the same measured layout and renderer. The neutral example in `examples/render_pickers.rs` creates nine dialogs and two destination pages. Optional second and third arguments supply HH:MM clocks for the Clean and Sketch exports; the project itself always uses local time when opened.

Focused tests cover calendar arithmetic, year/orientation layouts, strict button visibility, stable destinations, clipping, mode/size/history/persistence, component-local overrides, clock/date snapshot refresh and export generation. Native acceptance and exact typography/contour checks are recorded in the current checkpoint. Cross-platform execution and refresh cadence beyond the documented snapshot rules remain open.
