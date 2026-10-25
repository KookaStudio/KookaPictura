# Design

The stylesheet already owns the `QMenu` surface, so the radius, inset, and icon
offset live in the same rule block:

```css
QMenu { border: 1px solid ...; border-radius: 4px; padding: 2px; }
QMenu::icon { left: 10px; }
```

`QStyleSheetStyle` computes a menu icon's position from the item rule's padding
rect plus the icon rule's `left`, so `left` is the supported way to move the icon
without changing every action's pixmap. Every literal pixel length in the
template is scaled by the UI size before token substitution, so the radius and
offsets follow the UI-scale choice. The check-mark subcontrol uses the same
offset so check marks and icons stay aligned in one column.

A rounded radius alone is not enough: the popup window is opaque, and the
unpainted corner pixels would keep the window background and still look square.
`SpinArrowStyle` already wraps the application style, so its `polish` override
sets `WA_TranslucentBackground` on every `QMenu` before the window is created.
The 2 px inset keeps a selected first or last row from painting across the
rounded corner.

`QMenu#toolFlyoutMenu` keeps its own item padding; only the popup inset, radius,
and icon offset are shared.
