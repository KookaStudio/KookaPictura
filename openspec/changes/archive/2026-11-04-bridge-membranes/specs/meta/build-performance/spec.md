# Spec Delta

## MODIFIED Requirements

### Requirement: A bridge change rebuilds the C++ in the same build

When the `#[cxx_qt::bridge]` module of a bridge Rust source changes, the C++
translation units that include the generated cxx-qt headers — directly or through a
domain membrane header under `cpp/api/` — SHALL be recompiled by the same build that
regenerates those headers, and SHALL compile against the regenerated content. The
build system SHALL detect the bridge change before it decides what to compile, by
refreshing — at configure time — a stamp that records the content of the bridge
modules and that the C++ objects depend on. Because no translation unit outside
`cpp/api/` names a generated header path after the membrane migration, the dependency
SHALL be attached through the membrane headers, or through every translation unit that
includes a membrane or still names a generated header path (`pictura_app/src/`), so
adapter translation units keep the same-build guarantee. The stamp is global: any
bridge edit recompiles every membrane and adapter consumer; this change does not
change rebuild fan-out. The stamp SHALL cover, for each bridge file, the bridge module
together with the outer attributes and doc comments directly above it, and the file's
path, and nothing else in that file. An edit outside that text (an implementation
body, a helper, a test) SHALL NOT change the stamp and SHALL NOT recompile the C++.
The stamp SHALL record the whole file instead when it cannot find the bridge attribute
or the module's closing line, or when the text it extracted fails its consistency
check: unequal `{` and `}` counts, an odd number of unescaped `"`, or a `/*`
block-comment opener. A failed check therefore rebuilds too much rather than too
little. Adding or removing a Rust source, or changing the build script that lists the
bridge files, SHALL refresh the stamp. A build that follows a bridge change SHALL NOT
require a second build to pick the new interface up, and a build in which nothing
changed SHALL NOT recompile the C++.

#### Scenario: the bridge change lands in one build [bp_bridge_same_build]

- **WHEN** a bridge module in a bridge Rust source changes and the project is built once
- **THEN** the C++ translation units that include the generated header, directly or through a membrane or adapter header, are recompiled in that build, and the next build recompiles none of them

#### Scenario: an unchanged build recompiles nothing [bp_bridge_no_change]

- **WHEN** nothing under the bridge sources or the shell sources changed and the project is rebuilt
- **THEN** no C++ translation unit is recompiled

#### Scenario: an implementation edit in a bridge file recompiles no C++ [bp_bridge_impl_edit]

- **WHEN** a bridge Rust source changes only outside its `#[cxx_qt::bridge]` module and the attributes above it, and the project is rebuilt
- **THEN** the bridge stamp is unchanged and no C++ translation unit is recompiled

#### Scenario: a module the stamp cannot extract cleanly is hashed whole [bp_bridge_stamp_fallback]

- **WHEN** the stamp finds no closing line for a bridge module, or the extracted text has unbalanced braces, an odd number of unescaped quotes, or a `/*` (for example because a string or block comment inside the module holds a column-0 `}` line)
- **THEN** the stamp records the whole file, so any edit to that file rebuilds the dependent C++
