#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

// Opaque UI session state persisted across restarts. Not document data.
struct SessionState {
    QByteArray layout;               // QMainWindow::saveState()
    int layoutRevision = 0;          // chrome revision that wrote `layout`; a
                                     // mismatch discards the layout on restore
    bool quickMaskSelected = false;
    QString quickMaskColor = QStringLiteral("#ff0000");
    int quickMaskOpacity = 50;
    int layersThumbSize = 2;         // 0 None / 1 Small / 2 Medium / 3 Large
    int layersThumbContents = 0;     // 0 Entire Document / 1 Layer Bounds
    bool layersExpandNewEffects = true;
    bool layersAddCopyOnDuplicate = true;   // name a duplicate "<name> copy"
    bool layersUseDefaultMasksOnFill = true;  // fill layer takes selection mask
    bool layersAddMaskByDefault = true;       // new adjustment gets a mask
    int toolsColumns = 1;            // 1 or 2; out-of-range loads the default
    bool rulersVisible = false;
    bool guidesVisible = true;
    bool guidesLocked = false;
    // View > Show > Smart Guides: alignment guides and snapping while moving.
    bool smartGuidesVisible = true;
    // View > Show overlay state and the Extras master (frame_view.cpp).
    bool selectionEdges = true;
    bool showTargetPath = true;
    bool showLayerEdges = true;
    bool showGrid = false;
    bool showCount = true;
    bool showSlices = true;
    bool showNotes = true;
    bool showPixelGrid = true;
    bool showColorSamplers = true;
    bool extras = true;
    bool extrasMembersSet = false;
    QStringList extrasMembers;
    bool lockSlices = false;
    // Guides, Grid, & Slices: the Grid group.
    QString gridColor = QStringLiteral("#c0c0c0");
    int gridStyle = 0;    // 0 Lines, 1 Dashed Lines, 2 Dots
    int gridUnit = 1;     // 0 Pixels ... 6 Percent, matching the page combo
    double gridEvery = 1.0;
    int gridSubdivisions = 4;
    // View > Free Pan: free (default) vs the Photoshop-style constrained pan.
    bool freePan = true;
    // View > Snap Canvas To Workspace: pin a fitting canvas to the centre and
    // hide the workspace scrollbars. Default off.
    bool snapCanvasToWorkspace = false;
    // View > Proof Setup / Proof Colors: application view state (not PSD).
    // device 0 Working CMYK, 1 Working Gray, 2 sRGB, 3 Adobe RGB, 4 ProPhoto.
    int proofDevice = 0;
    int proofIntent = 1;
    bool proofBpc = true;
    bool proofSimulatePaper = false;
    bool proofSimulateBlack = false;
    bool proofColors = false;
    // The last Save Adobe PDF settings as canonical `.joboptions` text, minus
    // the passwords (empty until the first PDF save).
    QString pdfOptions;
    // The remembered per-format Save As options (jpeg/png/bmp/tiff/gif); an absent
    // key uses the CS6 defaults (`save_format_options.h`).
    QJsonObject saveFormatOptions;
    // The last accepted File > Automate > Batch options (the batch engine's
    // `BatchOptions` JSON); empty until the first batch.
    QJsonObject batchOptions;
    // The last accepted File > Scripts > Statistics mode and align choice
    // (`{"mode": int, "align": bool}`); empty until the first merge.
    QJsonObject statisticsOptions;
    // Crop options bar, persisted across restarts.
    bool cropClassicMode = false;    // Classic vs Modern (default) drag semantics
    bool cropDeletePixels = true;    // discard pixels outside the crop
    double cropRatio = 0.0;          // locked aspect ratio, 0 = unconstrained
    int cropGridOverlay = 0;         // 0 Rule of Thirds, 1 Grid, ...
    // Every other tool's options bar, by option group (tool_options_json.h).
    QJsonObject toolOptions;
    // Swatches panel contents as [{name, color}]. `swatchesLoaded` separates an
    // absent key (use the default palette) from an intentionally empty panel.
    QJsonArray swatches;
    bool swatchesLoaded = false;
    // Actions panel state. Expansion and selection are name-path identities
    // (set/action name plus ordinal, then step index); unknown identities are
    // ignored on restore.
    bool actionsButtonMode = false;
    bool actionsAllowToolRecording = false;
    QStringList actionsExpanded;
    QStringList actionsSelection;
    int actionsPlaybackMode = 0;     // 0 Accelerated / 1 Step By Step / 2 Pause
    double actionsPlaybackPause = 0.0;
    // v5-v8 panel-column state. `panelGroups` is the legacy flat JSON array of
    // {name, order, visible, minimized, collapsed} for the primary column;
    // `panelColumns` is the ordered array of
    // {side, order, width, railMode, groups:[...]}. Both are kept opaque here so
    // an older or newer store round-trips unchanged.
    QString panelRailMode = QStringLiteral("normal");
    int railWidth = 0;               // legacy primary width seed
    bool autoCollapseIconic = false;
    bool autoShowHidden = false;
    QJsonArray panelGroups;
    QJsonArray panelColumns;
    int schemaVersion = 12;
    QStringList recent;
    QByteArray windowGeometry;       // QMainWindow::saveGeometry(), empty = default
    bool windowMaximized = false;    // restored as Qt::WindowMaximized on launch
};

// Path of the session store, $XDG_STATE_HOME/kooka-pictura/state.json.
QString sessionFilePath();

// The caller holds the session lock for a read-modify-write.
QJsonObject readSessionObject();
bool writeSessionObject(const QJsonObject& object);
void setSessionCommitFailureForTest(bool fail);

// Load the session state, or defaults when the store is missing or corrupt.
SessionState loadSession();

// Write the session state atomically via QSaveFile. Returns false on failure.
bool saveSession(const SessionState& state);

// Whether a stored column other than the tools column holds groups.
bool sessionHasColumnLayout(const SessionState& state);

// Whether the session carries a panel layout: legacy groups, or a column with
// groups other than the tools column. A session without one is fresh.
bool sessionHasPanelLayout(const SessionState& state);

} // namespace pictura
