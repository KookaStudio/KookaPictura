# Test Plan

Covers both scenarios in the `meta/flatpak-release-build` delta. This change
only rewords the spec and refreshes three docs; no build, workflow, or packaging
code changes, so the behavior rows are verified manually against the shipped
manifest and the existing Flatpak dry run.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| The application builds from manifest sources only | Read the delta and `flatpak/io.github.KookaStudio.KookaPictura.yml`; confirm `flatpak-builder` fetches the manifest sources (the repository tree, `cargo-sources.json`, and the two archives), and the compile phase sets `CARGO_NET_OFFLINE=true` and passes `-DFETCHCONTENT_FULLY_DISCONNECTED=ON` | The spec separates the fetch from the offline compile; the manifest matches | manual: docs review against the manifest; the existing `flatpak.yml` Release-PR dry run exercises the build |
| CMake fetches nothing over the network | Read the manifest config opts and the unchanged scenario | The scenario matches `-DFETCHCONTENT_SOURCE_DIR_CXXQT`, `-DFETCHCONTENT_SOURCE_DIR_CORROSION`, and `-DFETCHCONTENT_FULLY_DISCONNECTED=ON` | manual: docs review against the manifest |
