#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>
#include <QtCore/QList>
#include <QtCore/QMap>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace pictura {

// The CS6 autosave interval choices, in minutes.
inline const QList<int> kAutoSaveMinuteChoices = {5, 10, 15, 30, 60};

// Opaque UI session state persisted across restarts. Not document data.
struct SessionState {
    QByteArray layout;               // QMainWindow::saveState()
    int layoutRevision = 0;          // chrome revision that wrote `layout`; a
                                     // mismatch discards the layout on restore
    int brightnessLevel = 1;         // Theme level
    int themeFamily = 0;             // Theme family: 0 dark / 1 light
    bool gpuCompute = true;          // GPU compositing preference
    int colorPolicy = 0;             // incoming RGB profile policy (0 Preserve)
    // Color Settings > More Options: Blend Text Colors Using Gamma (1.00–2.20).
    bool textGammaEnabled = true;
    double textGamma = 1.45;
    int layersThumbSize = 2;         // 0 None / 1 Small / 2 Medium / 3 Large
    int layersThumbContents = 0;     // 0 Entire Document / 1 Layer Bounds
    bool layersExpandNewEffects = true;
    bool layersAddCopyOnDuplicate = true;   // name a duplicate "<name> copy"
    bool layersUseDefaultMasksOnFill = true;  // fill/adjustment takes selection mask
    int toolsColumns = 1;            // 1 or 2; out-of-range loads the default
    bool useShiftKeyForToolSwitch = true;
    // Edit > Preferences > Type: new type starts with selected placeholder text.
    bool typePlaceholderText = true;
    bool confirmLiveShapeToPath = true;  // off after "Don't show again"
    bool confirmDeleteVectorMask = true;  // off after "Don't show again"
    bool showFullScreenMessage = true;   // off after "Don't show again"
    // Edit > Preferences > File Handling: Automatically Save Recovery
    // Information Every <n> minutes (CS6 choices 5/10/15/30/60, default 10).
    bool autoSaveRecovery = true;
    int autoSaveMinutes = 10;
    // View > Rulers / Show > Guides / Lock Guides, and the Guides, Grid, &
    // Slices guide colour (#rrggbb) and Dashed Lines style.
    bool rulersVisible = false;
    int rulerUnit = 1;               // RulerUnit; 1 Inches, CS6's default
    bool traditionalPoints = false;  // Units & Rulers: 72.27 rather than 72 pt/in
    bool guidesVisible = true;
    bool guidesLocked = false;
    // View > Show > Smart Guides: alignment guides and snapping while moving.
    bool smartGuidesVisible = true;
    // View > Free Pan: free (default) vs the Photoshop-style constrained pan.
    bool freePan = true;
    // View > Snap Canvas To Workspace: pin a fitting canvas to the centre and
    // hide the workspace scrollbars. Default off.
    bool snapCanvasToWorkspace = false;
    QString guideColor = QStringLiteral("#4affff");
    // The last Save Adobe PDF settings as canonical `.joboptions` text, minus
    // the passwords (empty until the first PDF save).
    QString pdfOptions;
    bool guideDashed = false;
    // Crop options bar, persisted across restarts.
    bool cropClassicMode = true;     // Classic (default) vs Modern drag semantics
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
    // Edit > Keyboard Shortcuts: only the differences from the default set.
    // Command id -> its bindings as portable key text (empty: none), and
    // tool name -> letter (empty: none).
    QMap<QString, QStringList> shortcuts;
    QMap<QString, QString> toolShortcuts;
    int schemaVersion = 10;
    QStringList recent;
    QByteArray windowGeometry;       // QMainWindow::saveGeometry(), empty = default
    bool windowMaximized = false;    // restored as Qt::WindowMaximized on launch
};

// Path of the session store, $XDG_STATE_HOME/kooka-pictura/state.json.
QString sessionFilePath();

// Load the session state, or defaults when the store is missing or corrupt.
SessionState loadSession();

// Write the session state atomically via QSaveFile. Returns false on failure.
bool saveSession(const SessionState& state);

} // namespace pictura
