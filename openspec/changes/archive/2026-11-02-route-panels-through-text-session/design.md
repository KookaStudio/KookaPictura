# Design

`TextSession::newTextCharacter` and `setNewTextCharacter` become `textCharacter` and
`setTextCharacter`, valid in any open session, and gain `textParagraph` and
`setTextParagraph`. The session holds panel edits in `character_` and `paragraph_`.
`currentSetting()` layers them as defaults, reopened layer, panel edits, then the
`TypeOptions` fields, so the options bar's fields stay in one place and commit needs no
change.

A panel writes back every field it shows, so `frame_build.cpp` refreshes both panels on
`typeOptionsChanged` while a session is open. `CharacterPanel::applying_` stops that echo
from rewriting the controls during the panel's own edit. The Paragraph panel needs no such
guard because its refresh blocks every field's signals.
