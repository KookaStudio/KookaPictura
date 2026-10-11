#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QJsonArray>
#include <QtCore/QList>
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
    bool gpuCompute = true;          // GPU compositing preference
    int colorPolicy = 0;             // incoming RGB profile policy (0 Preserve)
    int layersThumbSize = 2;         // 0 None / 1 Small / 2 Medium / 3 Large
    int layersThumbContents = 0;     // 0 Entire Document / 1 Layer Bounds
    bool layersExpandNewEffects = true;
    bool layersAddCopyOnDuplicate = true;   // name a duplicate "<name> copy"
    bool layersUseDefaultMasksOnFill = true;  // fill/adjustment takes selection mask
    int toolsColumns = 1;            // 1 or 2; out-of-range loads the default
    bool useShiftKeyForToolSwitch = true;
    bool confirmLiveShapeToPath = true;  // off after "Don't show again"
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
    QString guideColor = QStringLiteral("#4affff");
    bool guideDashed = false;
    // Crop options bar, persisted across restarts.
    bool cropClassicMode = true;     // Classic (default) vs Modern drag semantics
    bool cropDeletePixels = true;    // discard pixels outside the crop
    double cropRatio = 0.0;          // locked aspect ratio, 0 = unconstrained
    int cropGridOverlay = 0;         // 0 Rule of Thirds, 1 Grid, ...
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
    int schemaVersion = 9;
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
