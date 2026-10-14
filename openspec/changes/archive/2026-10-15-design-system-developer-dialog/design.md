# Design

## D1 — The browser is the app, not a mirror

The design system is expressed in QSS, Fusion, a `QProxyStyle` (the spin
arrows), and custom-painted widgets (`ColorSwatchWidget`, the graph panels, the
panel grips). None of that is reproducible in HTML/CSS, so any external mirror
is approximate by construction. The browser therefore builds every "story" from
the application's own widgets; the global stylesheet styles them exactly as it
styles the running UI. There is nothing to keep in sync and nothing to drift.

## D2 — Modeless tool window

`DesignSystemDialog` is a `QDialog` shown with `Qt::Tool` and `show()` (not the
app's blocking `runDialog`), so the rest of the application stays live: the user
can hover the real controls, open real menus, and see the real hover/pressed
states. The frame keeps one instance (`QPointer<DesignSystemDialog>`), reused on
each open.

## D3 — Story registry

`designSystemStories()` returns a vector of `DesignSystemStory{group, title,
blurb, tokens, primitive, build}`. `build` is a factory returning a `QWidget`
page; pages are built lazily on first selection and cached. Foundations pages
re-read `Theme::currentLevel()` when built, so switching brightness rebuilds the
open page (the token-coloured swatches) while palette-based pages recolour on
their own.

## D4 — Live brightness with restore

The brightness combo calls `Theme::apply(level)`, which recolours the whole
application immediately — the honest 1:1 preview. `Theme::currentLevel()` is
captured on open and re-applied in `closeEvent`, so a preview never changes the
user's persisted brightness preference.

## D5 — States

Rest, checked, disabled, and focus are forced on real instances
(`setChecked`/`setEnabled`/`setDown`/`setFocus`). Hover is forced with
`WA_UnderMouse` where it takes; otherwise the state is shown live by
interaction. The per-story "Show states" toggle hides the forced rows.

## D6 — Access

The command is declared in `command_tree.cpp` under `Help ▸ Developer ▸ Design
System…` (a nested path the menu builder already supports) and wired in
`frame_menus.cpp`. It is a deliberate, non-CS6 developer affordance documented
in the Help-menu notes.

## D7 — Testing

New GUI checks go in Qt Test (`cpp/tests/tst_design_system_dialog.cpp`), per the
GUI-test migration rule: the story list and groups, every page builds, the
curated primitives are present (`QPushButton`, `QToolButton`, `QLineEdit`,
`QPlainTextEdit`, `QSpinBox`, `QComboBox`, `QCheckBox`, `QRadioButton`,
`QSlider`, `QProgressBar`, `QGroupBox`, `QHeaderView`, `QScrollBar`, `QTabBar`,
`QMenu`, `QSplitter`, and the object-scoped `colorSwatch`, `numericField`,
`panelHeaderBand`), the filter narrows the list, brightness previews and
restores, and the pages render.
