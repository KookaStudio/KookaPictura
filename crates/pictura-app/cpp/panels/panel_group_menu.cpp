#include "panel_group.h"
#include <QtCore/QCoreApplication>

#include "actions_panel.h"
#include "commands.h"
#include "adjustment_catalogue.h"
#include "character_panel.h"
#include "panel_host.h"
#include "history_panel.h"
#include "layers_panel.h"
#include "paragraph_panel.h"
#include "properties_panel.h"

#include <QtCore/QHash>
#include <QtGui/QAction>
#include <QtWidgets/QMenu>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolButton>

#include <functional>
#include "swatches_panel.h"
#include "preset_store.h"
#include <QtCore/QFileInfo>

namespace pictura {

namespace {
// Phase D per-panel menu table, transcribed from `docs/dev/m42-panel-menus.md`
// in the researched order with the CS6 grouping separators. `action` non-empty
// means the entry is wired to a per-panel handler (via the panel's
// `performPanelMenuAction`); empty means it ships disabled under the
// "<label> — not implemented yet" tooltip convention.
struct MenuRow {
    QString label;
    QString action;
    bool checkable = false;
    bool submenu = false;
    bool separatorBefore = false;
    int depth = 0;
};

MenuRow dis(const QString& label, bool separatorBefore = false)
{
    return {label, QString(), false, false, separatorBefore, 0};
}
MenuRow chk(const QString& label, bool separatorBefore = false)
{
    return {label, QString(), true, false, separatorBefore, 0};
}
MenuRow chkAction(const QString& label, const QString& action, bool separatorBefore = false)
{
    return {label, action, true, false, separatorBefore, 0};
}
MenuRow imp(const QString& label, const QString& action, bool separatorBefore = false)
{
    return {label, action, false, false, separatorBefore, 0};
}
MenuRow sub(const QString& label, bool separatorBefore = false)
{
    return {label, QString(), false, true, separatorBefore, 0};
}
MenuRow kid(const QString& label, const QString& action = QString())
{
    return {label, action, false, false, false, 1};
}
MenuRow chkKid(const QString& label, const QString& action)
{
    MenuRow row = chkAction(label, action);
    row.depth = 1;
    return row;
}

const QList<MenuRow>& rowsForPanel(const QString& panel)
{
    static const QHash<QString, QList<MenuRow>> table = {
        {QStringLiteral("layersPanel"),
         {
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Layer…")), QStringLiteral("newLayer")),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Copy CSS"))),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Duplicate Layer/Group…")), QStringLiteral("duplicate"), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Delete Layer/Group")), QStringLiteral("delete")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Delete Hidden Layers")), QStringLiteral("deleteHidden")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Group…")), QStringLiteral("newGroup"), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Group from Layers…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Lock Layers…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Convert to Smart Object"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Rasterize Layer"))),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Group Layers")), QStringLiteral("group"), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Ungroup Layers")), QStringLiteral("ungroup")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Hide Layers")), QStringLiteral("hide")),
             sub(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Arrange")), true),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Bring to Front"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Bring Forward"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Send Backward"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Send to Back"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Reverse"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Move Layer Up")), QStringLiteral("moveUp")),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Move Layer Down")), QStringLiteral("moveDown")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Link Layers")), QStringLiteral("link"), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Select Linked Layers")), QStringLiteral("selectLinked")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Merge Down")), QStringLiteral("mergeDown"), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Merge Visible")), QStringLiteral("mergeVisible")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Merge Clipping Mask")), QStringLiteral("mergeClippingMask")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Flatten Image")), QStringLiteral("flatten"), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Blending Options…")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Animation Options"))),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Panel Options…")), QStringLiteral("panelOptions"), true),
         }},
        {QStringLiteral("channelsPanel"),
         {
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Channel…"))),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Duplicate Channel…")), QStringLiteral("duplicateChannel")),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Delete Channel"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Spot Channel…")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Merge Spot Channel(s)"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Split Channels")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Merge Channels…"))),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Channel Options…")), QStringLiteral("channelOptions")),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Panel Options…")), true),
         }},
        {QStringLiteral("pathsPanel"),
         {
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Path…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Duplicate Path…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Delete Path"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Save Path…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Make Work Path…")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Make Selection…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Fill Path…")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Fill Subpath…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Stroke Path…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Stroke Subpath…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Clipping Path…")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Panel Options…")), true),
         }},
        {QStringLiteral("colorPanel"),
         {
             sub(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Sliders"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Grayscale"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "RGB"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "HSB"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "CMYK"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Lab"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Web Color Sliders"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "RGB Spectrum")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "CMYK Spectrum"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Grayscale Ramp"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Current Colors"))),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Make Ramp Web Safe"))),
         }},
        {QStringLiteral("swatchesPanel"),
         {
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Swatch…"))),
             sub(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Display")), true),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Small Thumbnail"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Large Thumbnail"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Small List"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Large List"))),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Preset Manager…")), QStringLiteral("swatches.manager"), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Load Swatches…")), QStringLiteral("swatches.load")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Save Swatches…")), QStringLiteral("swatches.save")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Save Swatches For Exchange…")), QStringLiteral("swatches.exchange")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Replace Swatches…")), QStringLiteral("swatches.replace")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Reset Swatches")), QStringLiteral("swatches.reset")),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "(libraries list)")), true),
         }},
        {QStringLiteral("stylesPanel"),
         {
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Style…"))),
             sub(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Display")), true),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Text Only"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Small Thumbnail"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Large Thumbnail"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Small List"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Large List"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Preset Manager…")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Load Styles…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Save Styles…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Replace Styles…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Reset Styles"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "(libraries list)")), true),
         }},
        {QStringLiteral("navigatorPanel"),
         {
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Panel Options…"))),
         }},
        {QStringLiteral("histogramPanel"),
         {
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Uncached Refresh"))),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Compact View")), true),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Expanded View"))),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "All Channels View"))),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Show Channels In Color")), true),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Show Statistics"))),
         }},
        {QStringLiteral("infoPanel"),
         {
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Panel Options…"))),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Color Samplers")), true),
         }},
        // docs/02-ui-ux/panels/brushes-panel.md names these; the full CS6 list is
        // an open question there.
        {QStringLiteral("brushPanel"),
         {
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Brush Preset…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Rename Brush…"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Delete Brush"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Clear Brush Controls")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Copy Texture to Other Tools"))),
         }},
        {QStringLiteral("historyPanel"),
         {
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Step Forward")), QStringLiteral("stepForward")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Step Backward")), QStringLiteral("stepBackward")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Snapshot…")), QStringLiteral("newSnapshot"), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Delete"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Clear History"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Document")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "History Options…")), true),
         }},
        {QStringLiteral("actionsPanel"),
         {
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Button Mode")),
                       QStringLiteral("actions.buttonMode"), false),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Action…")), QStringLiteral("actions.newAction"), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "New Set…")), QStringLiteral("actions.newSet")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Duplicate")), QStringLiteral("actions.duplicate")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Delete")), QStringLiteral("actions.delete")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Play")), QStringLiteral("actions.play")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Start Recording")), QStringLiteral("actions.toggleRecording"),
                 true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Record Again…")), QStringLiteral("actions.recordAgain")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Insert Menu Item…")), QStringLiteral("actions.insertMenuItem"),
                 true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Insert Stop…")), QStringLiteral("actions.insertStop")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Insert Conditional…")),
                 QStringLiteral("actions.insertConditional")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Insert Path")), QStringLiteral("actions.insertPath")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Action Options…")), QStringLiteral("actions.options"), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Playback Options…")),
                 QStringLiteral("actions.playbackOptions")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Allow Tool Recording")),
                       QStringLiteral("actions.allowToolRecording")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Clear All Actions")), QStringLiteral("actions.clear"), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Reset Actions")), QStringLiteral("actions.reset")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Load Actions…")), QStringLiteral("actions.load")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Replace Actions…")), QStringLiteral("actions.replace")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Save Actions…")), QStringLiteral("actions.save")),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "(installed sets)")), true),
         }},
        {QStringLiteral("characterPanel"),
         {
             sub(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Change Text Orientation"))),
             kid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Standard Vertical Roman Alignment"))),
             sub(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "OpenType"))),
             chkKid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Standard Ligatures")),
                    QStringLiteral("character.standardLigatures")),
             chkKid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Contextual Alternates")),
                    QStringLiteral("character.contextualAlternates")),
             chkKid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Discretionary Ligatures")),
                    QStringLiteral("character.discretionaryLigatures")),
             chkKid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Swash")), QStringLiteral("character.swash")),
             chkKid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Stylistic Alternates")),
                    QStringLiteral("character.stylisticAlternates")),
             chkKid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Titling Alternates")),
                    QStringLiteral("character.titlingAlternates")),
             chkKid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Ordinals")), QStringLiteral("character.ordinals")),
             chkKid(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Fractions")), QStringLiteral("character.fractions")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Faux Bold")), QStringLiteral("character.fauxBold")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Faux Italic")), QStringLiteral("character.fauxItalic")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "All Caps")), QStringLiteral("character.allCaps")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Small Caps")), QStringLiteral("character.smallCaps")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Superscript")), QStringLiteral("character.superscript")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Subscript")), QStringLiteral("character.subscript")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Underline")), QStringLiteral("character.underline"), true),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Strikethrough")), QStringLiteral("character.strikethrough")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Fractional Widths")),
                       QStringLiteral("character.fractionalWidths"), true),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "System Layout")), QStringLiteral("character.systemLayout")),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "No Break")), true),
             sub(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Middle Eastern Features")), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Reset Character")), QStringLiteral("character.reset"), true),
         }},
        {QStringLiteral("paragraphPanel"),
         {
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Roman Hanging Punctuation")),
                       QStringLiteral("paragraph.hanging")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Justification…")), QStringLiteral("paragraph.justification"),
                 true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Hyphenation…"))),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Latin and East Asian Layout")), true),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "World-Ready Layout"))),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Default Features")), true),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "East Asian Features"))),
             chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Middle Eastern Features"))),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Single-line Composer")),
                       QStringLiteral("paragraph.singleLineComposer"), true),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Every-line Composer")),
                       QStringLiteral("paragraph.everyLineComposer")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Reset Paragraph")), QStringLiteral("paragraph.reset"), true),
         }},
        {QStringLiteral("adjustmentsPanel"),
         [] {
             QList<MenuRow> rows;
             for (const AdjustmentKind& entry : adjustmentCatalogue()) {
                 if (!entry.panelMenu.isEmpty()) {
                     rows << imp(entry.panelMenu, QStringLiteral("adjustment:") + entry.kind);
                 }
             }
             rows << chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Add Mask by Default")),
                                 QStringLiteral("addMaskByDefault"), true)
                  << chk(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Clip to Layer")));
             return rows;
         }()},
        {QStringLiteral("propertiesPanel"),
         {
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Save Preset…")), QStringLiteral("savePreset")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Load Preset…")), QStringLiteral("loadPreset")),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Save Black & White Preset"))),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Auto Options…")), QStringLiteral("autoOptions"), true),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Curves Display Options…")), QStringLiteral("properties.curvesDisplay")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Show Clipping For Black/White Points")), QStringLiteral("properties.showClipping"), true),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Auto-Select Parameter")), QStringLiteral("properties.autoParameter")),
             chkAction(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Auto-Select Targeted Adjustment Tool")), QStringLiteral("properties.autoTarget")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Delete Current Preset")), QStringLiteral("properties.deletePreset")),
             imp(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Reset Adjustment")), QStringLiteral("properties.reset")),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Apply Mask")), true),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Delete Mask"))),
             dis(QString::fromUtf8(QT_TRANSLATE_NOOP("PicturaUi", "Disable Mask"))),
         }},
    };
    static const QList<MenuRow> empty;
    const auto it = table.find(panel);
    return it == table.end() ? empty : it.value();
}

QString panelCommandId(const QString& panel, const QString& action)
{
    if (action.isEmpty()) {
        return {};
    }
    if (action.startsWith(QStringLiteral("adjustment:"))) {
        const QString kind = action.mid(11);
        for (const AdjustmentKind& entry : adjustmentCatalogue()) {
            if (entry.kind == kind) {
                return commandIdForPath({QStringLiteral("Layer"),
                                         QStringLiteral("New Adjustment Layer"), entry.title});
            }
        }
        return {};
    }
    if (panel == QStringLiteral("layersPanel")) {
        static const QHash<QString, const char*> map = {
            {QStringLiteral("newLayer"), command_ids::PanelLayersNewLayer},
            {QStringLiteral("duplicate"), command_ids::LayerDuplicateLayer},
            {QStringLiteral("delete"), command_ids::LayerDeleteLayer},
            {QStringLiteral("deleteHidden"), command_ids::LayerDeleteHiddenLayers},
            {QStringLiteral("newGroup"), command_ids::PanelLayersNewGroup},
            {QStringLiteral("group"), command_ids::LayerGroupLayers},
            {QStringLiteral("ungroup"), command_ids::LayerUngroupLayers},
            {QStringLiteral("hide"), command_ids::LayerHideLayers},
            {QStringLiteral("link"), command_ids::LayerLinkLayers},
            {QStringLiteral("selectLinked"), command_ids::LayerSelectLinked},
            {QStringLiteral("mergeDown"), command_ids::LayerMergeLayers},
            {QStringLiteral("mergeVisible"), command_ids::LayerMergeVisible},
            {QStringLiteral("mergeClippingMask"), command_ids::LayerMergeClippingMask},
            {QStringLiteral("flatten"), command_ids::LayerFlattenImage},
            {QStringLiteral("panelOptions"), command_ids::PanelLayersPanelOptions},
        };
        return map.contains(action) ? QString::fromLatin1(map.value(action)) : QString();
    }
    if (panel == QStringLiteral("channelsPanel")) {
        static const QHash<QString, const char*> map = {
            {QStringLiteral("duplicateChannel"), command_ids::PanelChannelsDuplicateChannel},
            {QStringLiteral("channelOptions"), command_ids::PanelChannelsChannelOptions},
        };
        return map.contains(action) ? QString::fromLatin1(map.value(action)) : QString();
    }
    if (panel == QStringLiteral("historyPanel")) {
        static const QHash<QString, const char*> map = {
            {QStringLiteral("stepForward"), command_ids::PanelHistoryStepForward},
            {QStringLiteral("stepBackward"), command_ids::PanelHistoryStepBackward},
            {QStringLiteral("newSnapshot"), command_ids::PanelHistoryNewSnapshot},
        };
        return map.contains(action) ? QString::fromLatin1(map.value(action)) : QString();
    }
    if (panel == QStringLiteral("propertiesPanel")) {
        static const QHash<QString, const char*> map = {
            {QStringLiteral("autoOptions"), command_ids::PanelPropertiesAutoOptions},
            {QStringLiteral("savePreset"), command_ids::PanelPropertiesSavePreset},
            {QStringLiteral("loadPreset"), command_ids::PanelPropertiesLoadPreset},
        };
        return map.contains(action) ? QString::fromLatin1(map.value(action)) : QString();
    }
    if (action.startsWith(QStringLiteral("actions."))
        || action.startsWith(QStringLiteral("character."))
        || action.startsWith(QStringLiteral("paragraph."))) {
        return action;
    }
    return {};
}

void populatePanelMenu(QMenu* menu, const QString& panelName,
                       const std::function<void(const QString&)>& dispatch,
                       CommandRegistry* registry)
{
    QMenu* submenu = nullptr;
    for (const MenuRow& row : rowsForPanel(panelName)) {
        const QString label = QCoreApplication::translate("PicturaUi", row.label.toUtf8().constData());
        if (row.depth == 0) {
            submenu = nullptr;
            if (row.separatorBefore) {
                menu->addSeparator();
            }
            if (row.submenu) {
                submenu = menu->addMenu(label);
                submenu->menuAction()->setEnabled(false);
                submenu->menuAction()->setToolTip(
                    QCoreApplication::translate("PicturaUi", "%1 — not implemented yet").arg(label));
                continue;
            }
            const QString command = panelCommandId(panelName, row.action);
            const bool preference = row.action == QStringLiteral("addMaskByDefault");
            QAction* action = nullptr;
            if (!command.isEmpty() && registry) {
                action = registry->detachedAction(command, menu);
                if (action) {
                    action->setCheckable(row.checkable || action->isCheckable());
                    menu->addAction(action);
                }
            }
            if (!action) {
                action = menu->addAction(label);
                action->setCheckable(row.checkable);
                if (!row.action.isEmpty()) {
                    action->setData(row.action);
                }
                if (row.label == QStringLiteral("(installed sets)")) {
                    action->setData(QStringLiteral("actions.installedPlaceholder"));
                }
                if (row.action.isEmpty()) {
                    action->setEnabled(false);
                    action->setToolTip(
                        QCoreApplication::translate("PicturaUi", "%1 — not implemented yet").arg(label));
                } else if (preference || command.isEmpty()) {
                    const QString id = row.action;
                    if (dispatch) {
                        QObject::connect(action, &QAction::triggered, menu,
                                         [dispatch, id]() { dispatch(id); });
                    }
                }
            }
            if (!command.isEmpty() && action && registry) {
                QObject::connect(action, &QAction::triggered, menu,
                                 [registry, command]() { registry->dispatch(command); });
            }
            continue;
        }
        if (!submenu) {
            continue;
        }
        const QString command = panelCommandId(panelName, row.action);
        QAction* action = nullptr;
        if (!command.isEmpty() && registry) {
            action = registry->detachedAction(command, submenu);
            if (action) {
                action->setCheckable(row.checkable || action->isCheckable());
                submenu->addAction(action);
            }
        }
        if (!action) {
            action = submenu->addAction(label);
            action->setCheckable(row.checkable);
            if (!row.action.isEmpty()) {
                action->setData(row.action);
            }
            if (row.action.isEmpty()) {
                action->setEnabled(false);
                action->setToolTip(QCoreApplication::translate("PicturaUi", "%1 — not implemented yet").arg(label));
            } else if (dispatch) {
                const QString id = row.action;
                QObject::connect(action, &QAction::triggered, menu,
                                 [dispatch, id]() { dispatch(id); });
            }
        } else {
            submenu->menuAction()->setEnabled(true);
            submenu->menuAction()->setToolTip(QString());
            QObject::connect(action, &QAction::triggered, menu,
                             [registry, command]() { registry->dispatch(command); });
        }
    }
    // M47: every live per-widget menu also closes the active tab and the whole
    // group. Only the live menu (with a dispatcher) carries them; the read-only
    // text listing stays the transcribed per-panel table.
    if (dispatch) {
        menu->addSeparator();
        QAction* closePanel = menu->addAction(PanelGroup::tr("Close"));
        closePanel->setData(QStringLiteral("close"));
        QObject::connect(closePanel, &QAction::triggered, menu,
                         [dispatch]() { dispatch(QStringLiteral("close")); });
        QAction* closeGroup = menu->addAction(PanelGroup::tr("Close Tab Group"));
        closeGroup->setData(QStringLiteral("closeGroup"));
        QObject::connect(closeGroup, &QAction::triggered, menu,
                         [dispatch]() { dispatch(QStringLiteral("closeGroup")); });
    }
}
} // namespace

void PanelGroup::updateHeaderMenu()
{
    if (!headerButton_) {
        return;
    }
    const QString panel = currentPanelName();
    // M49: the corner affordance is always present, even for a panel whose
    // transcribed menu is empty — it still offers Close / Close Tab Group.
    headerButton_->setVisible(true);
    if (headerCorner_) {
        headerCorner_->setVisible(true);
    }
    headerButton_->setMenu(nullptr);
    delete headerMenu_;
    headerMenu_ = nullptr;
    headerButton_->setObjectName(panel.isEmpty()
                                     ? QStringLiteral("panelWidgetMenu")
                                     : QStringLiteral("panelWidgetMenu_") + panel);
    headerButton_->setToolTip(panel.isEmpty()
                                  ? tr("Panel Menu")
                                  : tr("%1 Panel Menu").arg(titleForPanel(panel)));
    headerMenu_ = new QMenu(headerButton_);
    populatePanelMenu(headerMenu_, panel,
                      [this](const QString& id) { runPanelPreference(id); },
                      owningHost() ? owningHost()->commands() : nullptr);
    if (auto* swatches = qobject_cast<SwatchesPanel*>(tabs_->currentWidget())) {
        for (const auto& file : discoveredPresetFiles(PresetType::Swatches)) {
            auto* library = headerMenu_->addMenu(QFileInfo(file).completeBaseName());
            library->addAction(tr("Append"), swatches, [swatches, file] { swatches->runPresetCommand(QStringLiteral("append:") + file); });
            library->addAction(tr("Replace"), swatches, [swatches, file] { swatches->runPresetCommand(QStringLiteral("replace:") + file); });
        }
    }
    QObject::connect(headerMenu_, &QMenu::aboutToShow, headerMenu_, [this, panel] {
        // The menu is built once; re-read every command's state so a document
        // opened (or a selection changed) since then is reflected.
        if (PanelHost* host = owningHost()) {
            if (CommandRegistry* registry = host->commands()) {
                for (QAction* action : headerMenu_->findChildren<QAction*>()) {
                    registry->applyState(action);
                }
            }
        }
        if (panel == QStringLiteral("adjustmentsPanel")) {
            LayersPanel* layersPanel = qobject_cast<LayersPanel*>(tabs_->currentWidget());
            if (!layersPanel) {
                if (QWidget* win = window()) {
                    layersPanel =
                        win->findChild<LayersPanel*>(QStringLiteral("layersPanel"));
                }
            }
            for (QAction* action : headerMenu_->findChildren<QAction*>()) {
                if (action->data().toString() == QStringLiteral("addMaskByDefault")) {
                    action->setChecked(layersPanel && layersPanel->addMaskByDefault());
                }
            }
        }
        PanelMenuHook hook = panelMenuHooks_.value(panel);
        if (!hook) {
            if (PanelHost* host = owningHost()) {
                hook = host->panelMenuHook(panel);
            }
        }
        if (hook) {
            hook(headerMenu_);
        } else if (auto* actions = qobject_cast<ActionsPanel*>(tabs_->currentWidget())) {
            actions->updatePanelMenu(headerMenu_);
        }
    });
    headerButton_->setMenu(headerMenu_);
}

void PanelGroup::runPanelPreference(const QString& actionId)
{
    if (actionId.startsWith(QStringLiteral("swatches."))) {
        if (auto* panel = qobject_cast<SwatchesPanel*>(tabs_->currentWidget())) {
            panel->runPresetCommand(actionId);
        }
        return;
    }
    if (actionId == QStringLiteral("close")) {
        emit closePanelRequested(currentPanelName());
        return;
    }
    if (actionId == QStringLiteral("closeGroup")) {
        emit closeGroupRequested();
        return;
    }
    if (actionId == QStringLiteral("addMaskByDefault")) {
        if (QWidget* win = window()) {
            if (auto* layers = win->findChild<LayersPanel*>(QStringLiteral("layersPanel"))) {
                layers->performPanelMenuAction(actionId);
            }
        }
    }
}

bool PanelGroup::panelHasMenu(const QString& panelName)
{
    // The Clone Source spec lists no menu entries, so its live menu carries
    // only Close / Close Group.
    return !rowsForPanel(panelName).isEmpty() || panelName == QLatin1String("cloneSourcePanel");
}

QStringList PanelGroup::menuTextsForPanel(const QString& panelName)
{
    QMenu menu;
    populatePanelMenu(&menu, panelName, nullptr, nullptr);
    QStringList texts;
    for (QAction* action : menu.actions()) {
        if (!action->isSeparator()) {
            texts.push_back(action->text());
        }
    }
    return texts;
}

} // namespace pictura
