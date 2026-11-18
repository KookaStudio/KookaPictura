# Proposal

## Why

Kooka Pictura needs a version policy that protects documents written by newer readers and lets users exchange local PSD and PSB files with Photoshop CS6 and Creative Cloud. A Photoshop product version cannot tell Pictura whether an individual document can be edited or saved faithfully. [Issue #640](https://github.com/KookaStudio/KookaPictura/issues/640) · [Adobe PSD/PSB format specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/)

## What Changes

- Define the Kooka Pictura document version and minimum reader version, their storage in a private image resource and XMP, and what open and save do with missing, newer, or inconsistent metadata.
- Read PSD/PSB Version Info resource 1057, expose its fields, and keep the merged-image flag consistent on version-aware writes.
- Allow layered interchange with CS6 and CC when Pictura supports the document's features and can retain its opaque data. Warn and open read-only when compatibility prevents safe editing or native save. Use the source merged image where available and offer an explicitly confirmed flattened copy.
- Verify against local Photoshop-authored specimens from representative CS6, early CC, and current releases. Use repository-generated fixtures for automated tests.

## Capabilities

### New Capabilities

- `document/document-version-policy` defines private version metadata, feature-based access, and CS6/CC interchange acceptance.

### Modified Capabilities

- `codec/psd-image-resources` adds typed handling of Version Info 1057.
- `codec/psd-opaque-preservation` defines the preservation boundary for version-aware writes.
- `document/document-lifecycle` enforces the read-only state across editing and ordinary saves.
- `interop/save-a-copy` defines the warned flattened-copy flow for read-only documents.

## Impact

The change affects the PSD/PSB codec, metadata view, document open and save paths, command and tool mutation paths, and Qt tests. It targets local PSD/PSB interchange. Authoring additional CC-only features, Adobe cloud documents, and layered conversion to an older Photoshop feature set are outside the proposed scope.
