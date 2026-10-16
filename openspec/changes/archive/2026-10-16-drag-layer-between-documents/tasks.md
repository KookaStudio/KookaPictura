# Tasks: drag-layer-between-documents

## 1. Engine

- [x] 1.1 `copy_path_to_document` + `tests_transfer.rs`.

## 2. App

- [x] 2.1 `copy_layer_from_document` bridge; `PicturaMainWindow::copyLayerFromDocument`.
- [x] 2.2 Layer drags carry `kLayerSourceMimeType`; the tree and strip buttons ignore other documents' drags.
- [x] 2.3 `FileDropRouter`: tab hover activates the document; a tab or canvas drop copies the layer.

- [x] 2.4 Move tool: a canvas drag that reaches another document's tab becomes the layer drag (`FileDropRouter::otherDocumentTabAt`).

## 3. Verification

- [x] 3.1 `tst_layers_panel::dragLayerOntoAnotherDocument`, `moveToolFindsAnotherDocumentTab`.
- [x] 3.2 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
