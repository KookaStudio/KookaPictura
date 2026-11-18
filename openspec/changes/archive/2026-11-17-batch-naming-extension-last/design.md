# Design

`FileNaming::extension_is_last` ignores empty fields and checks that the last
extension field, if any, is the final one. `validate` applies it alongside the
unique-field rule, only for a Folder destination, the one destination that
uses the scheme. This is a deliberate narrowing of CS6 (marked `ponytail:` in
code): lifting it means saving in the step's `as` format whatever the suffix.
