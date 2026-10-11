// Command table for the documented CS6 menu tree (docs/02-ui-ux/menus.md).
#include "commands.h"

namespace pictura {
namespace {

// ponytail: unimplemented leaves get path-derived ids; swap in frozen
// command_ids once each gains a handler.
void leaf(CommandRegistry& registry, const QStringList& path, const QString& label,
          const QString& shortcut = QString()) {
    registry.add(commandIdForPath(path), path, label,
                 shortcut.isEmpty() ? QKeySequence() : QKeySequence(shortcut));
}

} // namespace

void addDefaultCommands(CommandRegistry& registry) {
    // File
    registry.add(command_ids::FileNew, {"File", "New…"}, QStringLiteral("New…"),
                 QKeySequence(QStringLiteral("Ctrl+N")), true);
    registry.add(command_ids::FileOpen, {"File", "Open..."}, QStringLiteral("Open..."),
                 QKeySequence(QStringLiteral("Ctrl+O")), true);
    leaf(registry, {"File", "Open As…"}, QStringLiteral("Open As…"),
         QStringLiteral("Ctrl+Alt+Shift+O"));
    registry.add(command_ids::FileOpenAsSmartObject, {"File", "Open As Smart Object…"},
                 QStringLiteral("Open As Smart Object…"), QKeySequence(), true);
    // The frame fills Open Recent each time it opens; this row is shown only
    // while the list is empty.
    leaf(registry, {"File", "Open Recent", "No Recent Files"}, QStringLiteral("No Recent Files"));
    registry.addSeparator({"File"});
    registry.add(command_ids::FileClose, {"File", "Close"}, QStringLiteral("Close"),
                 QKeySequence(QStringLiteral("Ctrl+W")), true);
    registry.add(command_ids::FileCloseAll, {"File", "Close All"}, QStringLiteral("Close All"),
                 QKeySequence(QStringLiteral("Ctrl+Alt+W")), true);
    registry.addSeparator({"File"});
    registry.add(command_ids::FileSave, {"File", "Save"}, QStringLiteral("Save"),
                 QKeySequence(QStringLiteral("Ctrl+S")), true);
    registry.add(command_ids::FileSaveAs, {"File", "Save As…"}, QStringLiteral("Save As…"),
                 QKeySequence(QStringLiteral("Shift+Ctrl+S")), true);
    registry.add(command_ids::FileExportAs, {"File", "Export As…"}, QStringLiteral("Export As…"),
                 QKeySequence(QStringLiteral("Ctrl+Alt+Shift+E")), true);
    registry.add(command_ids::FileQuickExportPng, {"File", "Quick Export as PNG"},
                 QStringLiteral("Quick Export as PNG"), QKeySequence(), true);
    leaf(registry, {"File", "Check In…"}, QStringLiteral("Check In…"));
    registry.add(command_ids::FileSaveForWeb, {"File", "Save for Web & Devices…"},
                 QStringLiteral("Save for Web & Devices…"),
                 QKeySequence(QStringLiteral("Ctrl+Alt+Shift+S")), true);
    registry.add(command_ids::FileRevert, {"File", "Revert…"}, QStringLiteral("Revert…"),
                 QKeySequence(QStringLiteral("F12")), true);
    registry.addSeparator({"File"});
    registry.add(command_ids::FilePlace, {"File", "Place…"}, QStringLiteral("Place…"),
                 QKeySequence(), true);
    leaf(registry, {"File", "Import", "Variable Data Sets"}, QStringLiteral("Variable Data Sets"));
    leaf(registry, {"File", "Import", "WIA Support"}, QStringLiteral("WIA Support"));
    leaf(registry, {"File", "Export", "Data Sets As Files…"}, QStringLiteral("Data Sets As Files…"));
    leaf(registry, {"File", "Export", "Render Video…"}, QStringLiteral("Render Video…"));
    leaf(registry, {"File", "Export", "Zoomify…"}, QStringLiteral("Zoomify…"));
    leaf(registry, {"File", "Automate", "Batch…"}, QStringLiteral("Batch…"));
    leaf(registry, {"File", "Automate", "Conditional Mode Change…"},
         QStringLiteral("Conditional Mode Change…"));
    leaf(registry, {"File", "Automate", "Contact Sheet II"}, QStringLiteral("Contact Sheet II"));
    leaf(registry, {"File", "Automate", "Create Droplet…"}, QStringLiteral("Create Droplet…"));
    leaf(registry, {"File", "Automate", "Crop And Straighten Photos"},
         QStringLiteral("Crop And Straighten Photos"));
    leaf(registry, {"File", "Automate", "Merge To HDR Pro…"}, QStringLiteral("Merge To HDR Pro…"));
    leaf(registry, {"File", "Automate", "PDF Presentation"}, QStringLiteral("PDF Presentation"));
    leaf(registry, {"File", "Automate", "Photomerge…"}, QStringLiteral("Photomerge…"));
    leaf(registry, {"File", "Automate", "Picture Package"}, QStringLiteral("Picture Package"));
    leaf(registry, {"File", "Scripts", "Export Layers to Files"},
         QStringLiteral("Export Layers to Files"));
    leaf(registry, {"File", "Scripts", "Image Processor"}, QStringLiteral("Image Processor"));
    leaf(registry, {"File", "Scripts", "Layer Comps to Files"}, QStringLiteral("Layer Comps to Files"));
    leaf(registry, {"File", "Scripts", "Load Files into Stack"}, QStringLiteral("Load Files into Stack"));
    leaf(registry, {"File", "Scripts", "Script Events Manager"}, QStringLiteral("Script Events Manager"));
    leaf(registry, {"File", "Scripts", "Statistics"}, QStringLiteral("Statistics"));
    registry.addSeparator({"File"});
    registry.add(command_ids::FileInfo, {"File", "File Info…"}, QStringLiteral("File Info…"),
                 QKeySequence(QStringLiteral("Ctrl+Alt+Shift+I")), true);
    leaf(registry, {"File", "Print…"}, QStringLiteral("Print…"), QStringLiteral("Ctrl+P"));
    leaf(registry, {"File", "Print One Copy"}, QStringLiteral("Print One Copy"),
         QStringLiteral("Ctrl+Alt+Shift+P"));
    registry.addSeparator({"File"});
    registry.add(command_ids::FileExit, {"File", "Exit"}, QStringLiteral("Exit"),
                 QKeySequence(QStringLiteral("Ctrl+Q")), true);

    // Edit
    registry.add(command_ids::EditUndo, {"Edit", "Undo"}, QStringLiteral("Undo"),
                 QKeySequence(QStringLiteral("Ctrl+Z")), true);
    registry.add(command_ids::EditRedo, {"Edit", "Redo"}, QStringLiteral("Redo"),
                 QKeySequence(QStringLiteral("Ctrl+Y")), true);
    registry.add(command_ids::EditStepForward, {"Edit", "Step Forward"},
                 QStringLiteral("Step Forward"), QKeySequence(QStringLiteral("Ctrl+Shift+Z")), true);
    registry.add(command_ids::EditStepBackward, {"Edit", "Step Backward"},
                 QStringLiteral("Step Backward"), QKeySequence(QStringLiteral("Ctrl+Alt+Z")), true);
    registry.addSeparator({"Edit"});
    leaf(registry, {"Edit", "Fade…"}, QStringLiteral("Fade…"), QStringLiteral("Shift+Ctrl+F"));
    registry.addSeparator({"Edit"});
    registry.add(command_ids::EditCut, {"Edit", "Cut"}, QStringLiteral("Cut"),
                 QKeySequence(QStringLiteral("Ctrl+X")), true);
    registry.add(command_ids::EditCopy, {"Edit", "Copy"}, QStringLiteral("Copy"),
                 QKeySequence(QStringLiteral("Ctrl+C")), true);
    registry.add(command_ids::EditCopyMerged, {"Edit", "Copy Merged"},
                 QStringLiteral("Copy Merged"), QKeySequence(QStringLiteral("Shift+Ctrl+C")), true);
    registry.add(command_ids::EditPaste, {"Edit", "Paste"}, QStringLiteral("Paste"),
                 QKeySequence(QStringLiteral("Ctrl+V")), true);
    // CS6 binds Shift+Ctrl+V to Paste in Place, but menus.md gives it to Paste
    // Into, so Paste in Place ships without a default shortcut.
    registry.add(command_ids::EditPasteInPlace, {"Edit", "Paste Special", "Paste in Place"},
                 QStringLiteral("Paste in Place"), QKeySequence(), true);
    registry.add(command_ids::EditPasteInto, {"Edit", "Paste Special", "Paste Into"},
                 QStringLiteral("Paste Into"), QKeySequence(QStringLiteral("Shift+Ctrl+V")), true);
    registry.add(command_ids::EditPasteOutside, {"Edit", "Paste Special", "Paste Outside"},
                 QStringLiteral("Paste Outside"), QKeySequence(), true);
    registry.add(command_ids::EditClear, {"Edit", "Clear"}, QStringLiteral("Clear"),
                 QKeySequence(), true);
    registry.addSeparator({"Edit"});
    leaf(registry, {"Edit", "Check Spelling…"}, QStringLiteral("Check Spelling…"));
    leaf(registry, {"Edit", "Find And Replace Text…"}, QStringLiteral("Find And Replace Text…"));
    registry.addSeparator({"Edit"});
    registry.add(command_ids::EditFill, {"Edit", "Fill…"}, QStringLiteral("Fill…"),
                 QKeySequence(QStringLiteral("Shift+F5")), true);
    registry.add(command_ids::EditStroke, {"Edit", "Stroke…"}, QStringLiteral("Stroke…"),
                 QKeySequence(), true);
    registry.addSeparator({"Edit"});
    // Photoshop's "Content-Aware Scale"; the UI uses a descriptive label so it
    // does not ship Adobe's coined feature name as our own.
    leaf(registry, {"Edit", "Seam-Aware Scale"}, QStringLiteral("Seam-Aware Scale"),
         QStringLiteral("Alt+Shift+Ctrl+C"));
    leaf(registry, {"Edit", "Puppet Warp"}, QStringLiteral("Puppet Warp"));
    registry.add(command_ids::EditFreeTransform, {"Edit", "Free Transform"},
                 QStringLiteral("Free Transform"), QKeySequence(QStringLiteral("Ctrl+T")), true);
    leaf(registry, {"Edit", "Transform", "Again"}, QStringLiteral("Again"),
         QStringLiteral("Shift+Ctrl+T"));
    leaf(registry, {"Edit", "Transform", "Scale"}, QStringLiteral("Scale"));
    leaf(registry, {"Edit", "Transform", "Rotate"}, QStringLiteral("Rotate"));
    registry.add(command_ids::EditTransformSkew, {"Edit", "Transform", "Skew"},
                 QStringLiteral("Skew"), QKeySequence(), true);
    registry.add(command_ids::EditTransformDistort, {"Edit", "Transform", "Distort"},
                 QStringLiteral("Distort"), QKeySequence(), true);
    registry.add(command_ids::EditTransformPerspective, {"Edit", "Transform", "Perspective"},
                 QStringLiteral("Perspective"), QKeySequence(), true);
    registry.add(command_ids::EditTransformWarp, {"Edit", "Transform", "Warp"},
                 QStringLiteral("Warp"), QKeySequence(), true);
    leaf(registry, {"Edit", "Transform", "Rotate 90° CW"}, QStringLiteral("Rotate 90° CW"));
    leaf(registry, {"Edit", "Transform", "Rotate 90° CCW"}, QStringLiteral("Rotate 90° CCW"));
    leaf(registry, {"Edit", "Transform", "Rotate 180°"}, QStringLiteral("Rotate 180°"));
    leaf(registry, {"Edit", "Transform", "Flip Horizontal"}, QStringLiteral("Flip Horizontal"));
    leaf(registry, {"Edit", "Transform", "Flip Vertical"}, QStringLiteral("Flip Vertical"));
    leaf(registry, {"Edit", "Transform Path", "Again"}, QStringLiteral("Again"));
    leaf(registry, {"Edit", "Transform Path", "Scale"}, QStringLiteral("Scale"));
    leaf(registry, {"Edit", "Transform Path", "Rotate"}, QStringLiteral("Rotate"));
    leaf(registry, {"Edit", "Transform Path", "Skew"}, QStringLiteral("Skew"));
    leaf(registry, {"Edit", "Transform Path", "Distort"}, QStringLiteral("Distort"));
    leaf(registry, {"Edit", "Transform Path", "Perspective"}, QStringLiteral("Perspective"));
    leaf(registry, {"Edit", "Transform Path", "Warp"}, QStringLiteral("Warp"));
    leaf(registry, {"Edit", "Transform Points", "Again"}, QStringLiteral("Again"));
    registry.addSeparator({"Edit"});
    leaf(registry, {"Edit", "Define Brush Preset…"}, QStringLiteral("Define Brush Preset…"));
    leaf(registry, {"Edit", "Define Pattern…"}, QStringLiteral("Define Pattern…"));
    leaf(registry, {"Edit", "Define Custom Shape…"}, QStringLiteral("Define Custom Shape…"));
    registry.addSeparator({"Edit"});
    registry.add(command_ids::EditPurgeUndo, {"Edit", "Purge", "Undo"},
                 QStringLiteral("Undo"), QKeySequence(), true);
    registry.add(command_ids::EditPurgeHistories, {"Edit", "Purge", "Histories"},
                 QStringLiteral("Histories"), QKeySequence(), true);
    registry.add(command_ids::EditPurgeClipboard, {"Edit", "Purge", "Clipboard"},
                 QStringLiteral("Clipboard"), QKeySequence(), true);
    leaf(registry, {"Edit", "Purge", "Video Cache"}, QStringLiteral("Video Cache"));
    registry.add(command_ids::EditPurgeAll, {"Edit", "Purge", "All"},
                 QStringLiteral("All"), QKeySequence(), true);
    registry.addSeparator({"Edit"});
    leaf(registry, {"Edit", "PDF Presets…"}, QStringLiteral("PDF Presets…"));
    leaf(registry, {"Edit", "Presets", "Preset Manager…"}, QStringLiteral("Preset Manager…"));
    leaf(registry, {"Edit", "Presets", "Migrate Presets…"}, QStringLiteral("Migrate Presets…"));
    leaf(registry, {"Edit", "Presets", "Export Presets…"}, QStringLiteral("Export Presets…"));
    leaf(registry, {"Edit", "Presets", "Import Presets…"}, QStringLiteral("Import Presets…"));
    registry.addSeparator({"Edit"});
    registry.add(command_ids::EditColorSettings, {"Edit", "Color Settings…"},
                 QStringLiteral("Color Settings…"), QKeySequence(QStringLiteral("Shift+Ctrl+K")),
                 true);
    registry.add(command_ids::EditAssignProfile, {"Edit", "Assign Profile…"},
                 QStringLiteral("Assign Profile…"), QKeySequence(), true);
    registry.add(command_ids::EditConvertProfile, {"Edit", "Convert to Profile…"},
                 QStringLiteral("Convert to Profile…"), QKeySequence(), true);
    registry.addSeparator({"Edit"});
    leaf(registry, {"Edit", "Keyboard Shortcuts…"}, QStringLiteral("Keyboard Shortcuts…"),
         QStringLiteral("Alt+Shift+Ctrl+K"));
    leaf(registry, {"Edit", "Menus…"}, QStringLiteral("Menus…"));
    registry.add(command_ids::EditPreferencesGeneral, {"Edit", "Preferences", "General"},
                 QStringLiteral("General"), QKeySequence(QStringLiteral("Ctrl+K")), true);
    registry.add(command_ids::EditPreferencesInterface, {"Edit", "Preferences", "Interface"},
                 QStringLiteral("Interface"), QKeySequence(), true);
    leaf(registry, {"Edit", "Preferences", "File Handling"}, QStringLiteral("File Handling"));
    registry.add(command_ids::EditPreferencesPerformance, {"Edit", "Preferences", "Performance"},
                 QStringLiteral("Performance"), QKeySequence(), true);
    leaf(registry, {"Edit", "Preferences", "Cursors"}, QStringLiteral("Cursors"));
    leaf(registry, {"Edit", "Preferences", "Transparency & Gamut"},
         QStringLiteral("Transparency & Gamut"));
    registry.add(command_ids::EditPreferencesUnits, {"Edit", "Preferences", "Units & Rulers"},
                 QStringLiteral("Units & Rulers"), QKeySequence(), true);
    registry.add(command_ids::EditPreferencesGuides, {"Edit", "Preferences", "Guides, Grid, & Slices"},
                 QStringLiteral("Guides, Grid, & Slices"), QKeySequence(), true);
    leaf(registry, {"Edit", "Preferences", "Plug-ins"}, QStringLiteral("Plug-ins"));
    leaf(registry, {"Edit", "Preferences", "Type"}, QStringLiteral("Type"));
    leaf(registry, {"Edit", "Preferences", "3D"}, QStringLiteral("3D"));

    // Image
    // Image ▸ Mode: the checked entry is the document's mode / depth.
    const auto mode = [&registry](const char* id, const char* name) {
        registry.add(CommandSpec{id, {"Image", "Mode", name}, QString::fromUtf8(name),
                                 QKeySequence(), true, true});
    };
    mode(command_ids::ImageModeBitmap, "Bitmap");
    mode(command_ids::ImageModeGrayscale, "Grayscale");
    leaf(registry, {"Image", "Mode", "Duotone"}, QStringLiteral("Duotone"));
    mode(command_ids::ImageModeIndexed, "Indexed Color");
    mode(command_ids::ImageModeRgb, "RGB Color");
    mode(command_ids::ImageModeCmyk, "CMYK Color");
    mode(command_ids::ImageModeLab, "Lab Color");
    leaf(registry, {"Image", "Mode", "Multichannel"}, QStringLiteral("Multichannel"));
    registry.addSeparator({"Image", "Mode"});
    mode(command_ids::ImageMode8Bits, "8 Bits/Channel");
    mode(command_ids::ImageMode16Bits, "16 Bits/Channel");
    mode(command_ids::ImageMode32Bits, "32 Bits/Channel");
    registry.addSeparator({"Image", "Mode"});
    leaf(registry, {"Image", "Mode", "Color Table"}, QStringLiteral("Color Table"));
    leaf(registry, {"Image", "Adjustments", "Brightness/Contrast"},
         QStringLiteral("Brightness/Contrast"));
    leaf(registry, {"Image", "Adjustments", "Levels"}, QStringLiteral("Levels"),
         QStringLiteral("Ctrl+L"));
    leaf(registry, {"Image", "Adjustments", "Curves"}, QStringLiteral("Curves"),
         QStringLiteral("Ctrl+M"));
    leaf(registry, {"Image", "Adjustments", "Exposure"}, QStringLiteral("Exposure"));
    leaf(registry, {"Image", "Adjustments", "Vibrance"}, QStringLiteral("Vibrance"));
    leaf(registry, {"Image", "Adjustments", "Hue/Saturation"}, QStringLiteral("Hue/Saturation"),
         QStringLiteral("Ctrl+U"));
    leaf(registry, {"Image", "Adjustments", "Color Balance"}, QStringLiteral("Color Balance"),
         QStringLiteral("Ctrl+B"));
    leaf(registry, {"Image", "Adjustments", "Black & White"}, QStringLiteral("Black & White"),
         QStringLiteral("Alt+Shift+Ctrl+B"));
    leaf(registry, {"Image", "Adjustments", "Photo Filter"}, QStringLiteral("Photo Filter"));
    leaf(registry, {"Image", "Adjustments", "Channel Mixer"}, QStringLiteral("Channel Mixer"));
    leaf(registry, {"Image", "Adjustments", "Color Lookup"}, QStringLiteral("Color Lookup"));
    registry.addSeparator({"Image", "Adjustments"});
    leaf(registry, {"Image", "Adjustments", "Invert"}, QStringLiteral("Invert"),
         QStringLiteral("Ctrl+I"));
    leaf(registry, {"Image", "Adjustments", "Posterize"}, QStringLiteral("Posterize"));
    leaf(registry, {"Image", "Adjustments", "Threshold"}, QStringLiteral("Threshold"));
    leaf(registry, {"Image", "Adjustments", "Gradient Map"}, QStringLiteral("Gradient Map"));
    leaf(registry, {"Image", "Adjustments", "Selective Color"}, QStringLiteral("Selective Color"));
    registry.addSeparator({"Image", "Adjustments"});
    leaf(registry, {"Image", "Adjustments", "Shadows/Highlights"},
         QStringLiteral("Shadows/Highlights"));
    leaf(registry, {"Image", "Adjustments", "HDR Toning"}, QStringLiteral("HDR Toning"));
    registry.addSeparator({"Image", "Adjustments"});
    leaf(registry, {"Image", "Adjustments", "Desaturate"}, QStringLiteral("Desaturate"),
         QStringLiteral("Shift+Ctrl+U"));
    leaf(registry, {"Image", "Adjustments", "Match Color"}, QStringLiteral("Match Color"));
    leaf(registry, {"Image", "Adjustments", "Replace Color"}, QStringLiteral("Replace Color"));
    leaf(registry, {"Image", "Adjustments", "Equalize"}, QStringLiteral("Equalize"));
    registry.addSeparator({"Image"});
    leaf(registry, {"Image", "Auto Tone"}, QStringLiteral("Auto Tone"), QStringLiteral("Shift+Ctrl+L"));
    leaf(registry, {"Image", "Auto Contrast"}, QStringLiteral("Auto Contrast"),
         QStringLiteral("Alt+Shift+Ctrl+L"));
    leaf(registry, {"Image", "Auto Color"}, QStringLiteral("Auto Color"), QStringLiteral("Shift+Ctrl+B"));
    registry.addSeparator({"Image"});
    leaf(registry, {"Image", "Image Size…"}, QStringLiteral("Image Size…"),
         QStringLiteral("Ctrl+Alt+I"));
    leaf(registry, {"Image", "Canvas Size…"}, QStringLiteral("Canvas Size…"),
         QStringLiteral("Ctrl+Alt+C"));
    registry.addSeparator({"Image"});
    registry.add(command_ids::ImageRotate180, {"Image", "Image Rotation", "180°"},
                 QStringLiteral("180°"), QKeySequence(), true);
    registry.add(command_ids::ImageRotate90Cw, {"Image", "Image Rotation", "90° CW"},
                 QStringLiteral("90° CW"), QKeySequence(), true);
    registry.add(command_ids::ImageRotate90Ccw, {"Image", "Image Rotation", "90° CCW"},
                 QStringLiteral("90° CCW"), QKeySequence(), true);
    registry.add(command_ids::ImageRotateArbitrary, {"Image", "Image Rotation", "Arbitrary…"},
                 QStringLiteral("Arbitrary…"), QKeySequence(), true);
    registry.add(command_ids::ImageFlipHorizontal, {"Image", "Image Rotation", "Flip Canvas Horizontal"},
                 QStringLiteral("Flip Canvas Horizontal"), QKeySequence(), true);
    registry.add(command_ids::ImageFlipVertical, {"Image", "Image Rotation", "Flip Canvas Vertical"},
                 QStringLiteral("Flip Canvas Vertical"), QKeySequence(), true);
    registry.addSeparator({"Image"});
    registry.add(command_ids::ImageCrop, {"Image", "Crop"}, QStringLiteral("Crop"),
                 QKeySequence(), true);
    registry.add(command_ids::ImageTrim, {"Image", "Trim…"}, QStringLiteral("Trim…"),
                 QKeySequence(), true);
    registry.add(command_ids::ImageDuplicate, {"Image", "Duplicate…"},
                 QStringLiteral("Duplicate…"), QKeySequence(), true);
    registry.addSeparator({"Image"});
    leaf(registry, {"Image", "Variables", "Define…"}, QStringLiteral("Define…"));
    leaf(registry, {"Image", "Variables", "Data Sets…"}, QStringLiteral("Data Sets…"));
    leaf(registry, {"Image", "Apply Data Set…"}, QStringLiteral("Apply Data Set…"));
    registry.addSeparator({"Image"});
    leaf(registry, {"Image", "Trap…"}, QStringLiteral("Trap…"));
    leaf(registry, {"Image", "Apply Image…"}, QStringLiteral("Apply Image…"));
    leaf(registry, {"Image", "Calculations…"}, QStringLiteral("Calculations…"));

    // Layer
    registry.add(command_ids::LayerNewLayer, {"Layer", "New", "Layer"}, QStringLiteral("Layer"),
                 QKeySequence(QStringLiteral("Shift+Ctrl+N")), true);
    registry.add(command_ids::LayerNewLayerFromBackground,
                 {"Layer", "New", "Layer from Background…"},
                 QStringLiteral("Layer from Background…"), QKeySequence(), true);
    registry.add(command_ids::LayerNewGroup, {"Layer", "New", "Group…"}, QStringLiteral("Group…"),
                 QKeySequence(), true);
    registry.add(command_ids::LayerNewGroupFromLayers, {"Layer", "New", "Group from Layers…"},
                 QStringLiteral("Group from Layers…"), QKeySequence(), true);
    registry.add(command_ids::LayerNewLayerViaCopy, {"Layer", "New", "Layer via Copy"},
                 QStringLiteral("Layer via Copy"), QKeySequence(QStringLiteral("Ctrl+J")), true);
    registry.add(command_ids::LayerNewLayerViaCut, {"Layer", "New", "Layer via Cut"},
                 QStringLiteral("Layer via Cut"),
                 QKeySequence(QStringLiteral("Shift+Ctrl+J")), true);
    registry.add(command_ids::LayerNewBackgroundFromLayer,
                 {"Layer", "New", "Background From Layer"},
                 QStringLiteral("Background From Layer"), QKeySequence(), true);
    registry.addSeparator({"Layer"});
    registry.add(command_ids::LayerDuplicateLayer, {"Layer", "Duplicate Layer…"},
                 QStringLiteral("Duplicate Layer…"), QKeySequence(), true);
    registry.add(command_ids::LayerDeleteLayer, {"Layer", "Delete Layer"},
                 QStringLiteral("Delete Layer"), QKeySequence(), true);
    registry.add(command_ids::LayerDeleteHiddenLayers, {"Layer", "Delete Hidden Layers"},
                 QStringLiteral("Delete Hidden Layers"), QKeySequence(), true);
    registry.addSeparator({"Layer"});
    leaf(registry, {"Layer", "Layer Style", "Blending Options…"},
         QStringLiteral("Blending Options…"));
    leaf(registry, {"Layer", "Layer Style", "Drop Shadow…"}, QStringLiteral("Drop Shadow…"));
    leaf(registry, {"Layer", "Layer Style", "Inner Shadow…"}, QStringLiteral("Inner Shadow…"));
    leaf(registry, {"Layer", "Layer Style", "Outer Glow…"}, QStringLiteral("Outer Glow…"));
    leaf(registry, {"Layer", "Layer Style", "Inner Glow…"}, QStringLiteral("Inner Glow…"));
    leaf(registry, {"Layer", "Layer Style", "Bevel & Emboss…"}, QStringLiteral("Bevel & Emboss…"));
    leaf(registry, {"Layer", "Layer Style", "Satin…"}, QStringLiteral("Satin…"));
    leaf(registry, {"Layer", "Layer Style", "Color Overlay…"}, QStringLiteral("Color Overlay…"));
    leaf(registry, {"Layer", "Layer Style", "Gradient Overlay…"},
         QStringLiteral("Gradient Overlay…"));
    leaf(registry, {"Layer", "Layer Style", "Pattern Overlay…"}, QStringLiteral("Pattern Overlay…"));
    leaf(registry, {"Layer", "Layer Style", "Stroke…"}, QStringLiteral("Stroke…"));
    registry.addSeparator({"Layer", "Layer Style"});
    leaf(registry, {"Layer", "Layer Style", "Copy Layer Style"}, QStringLiteral("Copy Layer Style"));
    leaf(registry, {"Layer", "Layer Style", "Paste Layer Style"},
         QStringLiteral("Paste Layer Style"));
    leaf(registry, {"Layer", "Layer Style", "Clear Layer Style"},
         QStringLiteral("Clear Layer Style"));
    leaf(registry, {"Layer", "Layer Style", "Global Light…"}, QStringLiteral("Global Light…"));
    leaf(registry, {"Layer", "Layer Style", "Create Layers"}, QStringLiteral("Create Layers"));
    leaf(registry, {"Layer", "Layer Style", "Scale Effects"}, QStringLiteral("Scale Effects"));
    leaf(registry, {"Layer", "Layer Style", "Hide All Effects"}, QStringLiteral("Hide All Effects"));
    leaf(registry, {"Layer", "Layer Style", "Show All Effects"}, QStringLiteral("Show All Effects"));
    leaf(registry, {"Layer", "Smart Filter", "Disable Filter Mask"},
         QStringLiteral("Disable Filter Mask"));
    leaf(registry, {"Layer", "Smart Filter", "Delete Filter Mask"},
         QStringLiteral("Delete Filter Mask"));
    registry.add(command_ids::LayerSmartFiltersClear, {"Layer", "Smart Filter", "Clear Smart Filters"},
                 QStringLiteral("Clear Smart Filters"), QKeySequence(), true);
    registry.addSeparator({"Layer"});
    registry.add(command_ids::LayerNewFillSolidColor, {"Layer", "New Fill Layer", "Solid Color…"},
                 QStringLiteral("Solid Color…"), QKeySequence(), true);
    registry.add(command_ids::LayerNewFillGradient, {"Layer", "New Fill Layer", "Gradient…"},
                 QStringLiteral("Gradient…"), QKeySequence(), true);
    leaf(registry, {"Layer", "New Fill Layer", "Pattern…"}, QStringLiteral("Pattern…"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Brightness/Contrast"},
         QStringLiteral("Brightness/Contrast"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Levels"}, QStringLiteral("Levels"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Curves"}, QStringLiteral("Curves"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Exposure"}, QStringLiteral("Exposure"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Vibrance"}, QStringLiteral("Vibrance"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Hue/Saturation"},
         QStringLiteral("Hue/Saturation"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Color Balance"},
         QStringLiteral("Color Balance"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Black & White"},
         QStringLiteral("Black & White"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Photo Filter"}, QStringLiteral("Photo Filter"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Channel Mixer"},
         QStringLiteral("Channel Mixer"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Color Lookup"}, QStringLiteral("Color Lookup"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Invert"}, QStringLiteral("Invert"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Posterize"}, QStringLiteral("Posterize"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Threshold"}, QStringLiteral("Threshold"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Gradient Map"}, QStringLiteral("Gradient Map"));
    leaf(registry, {"Layer", "New Adjustment Layer", "Selective Color"},
         QStringLiteral("Selective Color"));
    registry.add(command_ids::LayerContentOptions, {"Layer", "Layer Content Options…"},
                 QStringLiteral("Layer Content Options…"), QKeySequence(), true);
    registry.addSeparator({"Layer"});
    registry.add(command_ids::LayerMaskRevealAll, {"Layer", "Layer Mask", "Reveal All"},
                 QStringLiteral("Reveal All"), QKeySequence(), true);
    registry.add(command_ids::LayerMaskHideAll, {"Layer", "Layer Mask", "Hide All"},
                 QStringLiteral("Hide All"), QKeySequence(), true);
    registry.add(command_ids::LayerMaskRevealSelection,
                 {"Layer", "Layer Mask", "Reveal Selection"}, QStringLiteral("Reveal Selection"),
                 QKeySequence(), true);
    registry.add(command_ids::LayerMaskHideSelection, {"Layer", "Layer Mask", "Hide Selection"},
                 QStringLiteral("Hide Selection"), QKeySequence(), true);
    registry.add(command_ids::LayerMaskFromTransparency,
                 {"Layer", "Layer Mask", "From Transparency"},
                 QStringLiteral("From Transparency"), QKeySequence(), true);
    registry.add(command_ids::LayerMaskDelete, {"Layer", "Layer Mask", "Delete"},
                 QStringLiteral("Delete"), QKeySequence(), true);
    registry.add(command_ids::LayerMaskApply, {"Layer", "Layer Mask", "Apply"},
                 QStringLiteral("Apply"), QKeySequence(), true);
    registry.add(command_ids::LayerMaskEnable, {"Layer", "Layer Mask", "Enable"},
                 QStringLiteral("Enable"), QKeySequence(), true);
    registry.add(command_ids::LayerMaskDisable, {"Layer", "Layer Mask", "Disable"},
                 QStringLiteral("Disable"), QKeySequence(), true);
    registry.add(command_ids::LayerMaskLink, {"Layer", "Layer Mask", "Link"},
                 QStringLiteral("Link"), QKeySequence(), true);
    registry.add(command_ids::LayerMaskUnlink, {"Layer", "Layer Mask", "Unlink"},
                 QStringLiteral("Unlink"), QKeySequence(), true);
    registry.add(command_ids::LayerVectorMaskRevealAll, {"Layer", "Vector Mask", "Reveal All"},
                 QStringLiteral("Reveal All"), QKeySequence(), true);
    registry.add(command_ids::LayerVectorMaskHideAll, {"Layer", "Vector Mask", "Hide All"},
                 QStringLiteral("Hide All"), QKeySequence(), true);
    registry.add(command_ids::LayerVectorMaskCurrentPath, {"Layer", "Vector Mask", "Current Path"},
                 QStringLiteral("Current Path"), QKeySequence(), true);
    registry.add(command_ids::LayerVectorMaskDelete, {"Layer", "Vector Mask", "Delete"},
                 QStringLiteral("Delete"), QKeySequence(), true);
    registry.add(command_ids::LayerVectorMaskEnable, {"Layer", "Vector Mask", "Enable"},
                 QStringLiteral("Enable"), QKeySequence(), true);
    registry.add(command_ids::LayerVectorMaskDisable, {"Layer", "Vector Mask", "Disable"},
                 QStringLiteral("Disable"), QKeySequence(), true);
    registry.add(command_ids::LayerVectorMaskLink, {"Layer", "Vector Mask", "Link"},
                 QStringLiteral("Link"), QKeySequence(), true);
    registry.add(command_ids::LayerVectorMaskUnlink, {"Layer", "Vector Mask", "Unlink"},
                 QStringLiteral("Unlink"), QKeySequence(), true);
    registry.add(command_ids::LayerCreateClippingMask, {"Layer", "Create Clipping Mask"},
                 QStringLiteral("Create Clipping Mask"), QKeySequence(QStringLiteral("Ctrl+Alt+G")),
                 true);
    registry.add(command_ids::LayerReleaseClippingMask, {"Layer", "Release Clipping Mask"},
                 QStringLiteral("Release Clipping Mask"), QKeySequence(), true);
    registry.addSeparator({"Layer"});
    registry.add(command_ids::LayerSmartObjectConvertTo,
                 {"Layer", "Smart Objects", "Convert to Smart Object"},
                 QStringLiteral("Convert to Smart Object"), QKeySequence(), true);
    registry.add(command_ids::LayerSmartObjectNewViaCopy,
                 {"Layer", "Smart Objects", "New Smart Object via Copy"},
                 QStringLiteral("New Smart Object via Copy"), QKeySequence(), true);
    registry.add(command_ids::LayerSmartObjectEditContents,
                 {"Layer", "Smart Objects", "Edit Contents"},
                 QStringLiteral("Edit Contents"), QKeySequence(), true);
    registry.add(command_ids::LayerSmartObjectExportContents,
                 {"Layer", "Smart Objects", "Export Contents…"},
                 QStringLiteral("Export Contents…"), QKeySequence(), true);
    registry.add(command_ids::LayerSmartObjectReplaceContents,
                 {"Layer", "Smart Objects", "Replace Contents…"},
                 QStringLiteral("Replace Contents…"), QKeySequence(), true);
    registry.add(command_ids::LayerSmartObjectResetTransform,
                 {"Layer", "Smart Objects", "Reset Transform"},
                 QStringLiteral("Reset Transform"), QKeySequence(), true);
    registry.add(command_ids::LayerSmartObjectConvertToLayers,
                 {"Layer", "Smart Objects", "Convert to Layers"},
                 QStringLiteral("Convert to Layers"), QKeySequence(), true);
    leaf(registry, {"Layer", "Smart Objects", "Rasterize"}, QStringLiteral("Rasterize"));
    leaf(registry, {"Layer", "Smart Objects", "Stack Mode"}, QStringLiteral("Stack Mode"));
    leaf(registry, {"Layer", "Video Layers", "New Blank Video Layer"},
         QStringLiteral("New Blank Video Layer"));
    leaf(registry, {"Layer", "Video Layers", "New Video Layer From File"},
         QStringLiteral("New Video Layer From File"));
    leaf(registry, {"Layer", "Video Layers", "Replace Footage"}, QStringLiteral("Replace Footage"));
    leaf(registry, {"Layer", "Video Layers", "Interpret Footage"},
         QStringLiteral("Interpret Footage"));
    // Rasterize: Fill Content, Type, Shape, Layer, Smart Object, and Vector Mask
    // have a model kind to rasterize. Layer Style, Video, and 3D stay disabled
    // because their layer kinds do not exist; `All Layers` rasterizes fill
    // content, shape, and type layers.
    registry.add(command_ids::LayerRasterizeType, {"Layer", "Rasterize", "Type"},
                 QStringLiteral("Type"), QKeySequence(), true);
    registry.add(command_ids::LayerRasterizeShape, {"Layer", "Rasterize", "Shape"},
                 QStringLiteral("Shape"), QKeySequence(), true);
    registry.add(command_ids::LayerRasterizeVectorMask, {"Layer", "Rasterize", "Vector Mask"},
                 QStringLiteral("Vector Mask"), QKeySequence(), true);
    registry.add(command_ids::LayerRasterizeSmartObject, {"Layer", "Rasterize", "Smart Object"},
                 QStringLiteral("Smart Object"), QKeySequence(), true);
    registry.add(command_ids::LayerRasterizeFillContent, {"Layer", "Rasterize", "Fill Content"},
                 QStringLiteral("Fill Content"), QKeySequence(), true);
    registry.add(command_ids::LayerRasterizeLayer, {"Layer", "Rasterize", "Layer"},
                 QStringLiteral("Layer"), QKeySequence(), true);
    registry.add(command_ids::LayerRasterizeAllLayers, {"Layer", "Rasterize", "All Layers"},
                 QStringLiteral("All Layers"), QKeySequence(), true);
    leaf(registry, {"Layer", "Rasterize", "Layer Style"}, QStringLiteral("Layer Style"));
    leaf(registry, {"Layer", "Rasterize", "Video"}, QStringLiteral("Video"));
    leaf(registry, {"Layer", "Rasterize", "3D"}, QStringLiteral("3D"));
    registry.addSeparator({"Layer"});
    registry.add(command_ids::LayerCopyShapeAttributes, {"Layer", "Copy Shape Attributes"},
                 QStringLiteral("Copy Shape Attributes"), QKeySequence(), true);
    registry.add(command_ids::LayerPasteShapeAttributes, {"Layer", "Paste Shape Attributes"},
                 QStringLiteral("Paste Shape Attributes"), QKeySequence(), true);
    leaf(registry, {"Layer", "New Layer-based Slice"}, QStringLiteral("New Layer-based Slice"));
    registry.addSeparator({"Layer"});
    registry.add(command_ids::LayerGroupLayers, {"Layer", "Group Layers"},
                 QStringLiteral("Group Layers"), QKeySequence(QStringLiteral("Ctrl+G")), true);
    registry.add(command_ids::LayerUngroupLayers, {"Layer", "Ungroup Layers"},
                 QStringLiteral("Ungroup Layers"),
                 QKeySequence(QStringLiteral("Shift+Ctrl+G")), true);
    registry.add(command_ids::LayerHideLayers, {"Layer", "Hide Layers"},
                 QStringLiteral("Hide Layers"), QKeySequence(), true);
    registry.add(command_ids::LayerSelectSimilar, {"Layer", "Select Similar Layers"},
                 QStringLiteral("Select Similar Layers"), QKeySequence(), true);
    registry.add(command_ids::LayerLinkLayers, {"Layer", "Link Layers"},
                 QStringLiteral("Link Layers"), QKeySequence(), true);
    registry.add(command_ids::LayerUnlinkLayers, {"Layer", "Unlink Layers"},
                 QStringLiteral("Unlink Layers"), QKeySequence(), true);
    registry.add(command_ids::LayerSelectLinked, {"Layer", "Select Linked Layers"},
                 QStringLiteral("Select Linked Layers"), QKeySequence(), true);
    registry.add(command_ids::LayerArrangeFront, {"Layer", "Arrange", "Bring to Front"},
                 QStringLiteral("Bring to Front"), QKeySequence(QStringLiteral("Shift+Ctrl+]")),
                 true);
    registry.add(command_ids::LayerArrangeForward, {"Layer", "Arrange", "Bring Forward"},
                 QStringLiteral("Bring Forward"), QKeySequence(QStringLiteral("Ctrl+]")), true);
    registry.add(command_ids::LayerArrangeBackward, {"Layer", "Arrange", "Send Backward"},
                 QStringLiteral("Send Backward"), QKeySequence(QStringLiteral("Ctrl+[")), true);
    registry.add(command_ids::LayerArrangeBack, {"Layer", "Arrange", "Send to Back"},
                 QStringLiteral("Send to Back"), QKeySequence(QStringLiteral("Shift+Ctrl+[")),
                 true);
    registry.add(command_ids::LayerArrangeReverse, {"Layer", "Arrange", "Reverse"},
                 QStringLiteral("Reverse"), QKeySequence(), true);
    leaf(registry, {"Layer", "Align", "Top"}, QStringLiteral("Top"));
    leaf(registry, {"Layer", "Align", "Vertical Center"}, QStringLiteral("Vertical Center"));
    leaf(registry, {"Layer", "Align", "Bottom"}, QStringLiteral("Bottom"));
    leaf(registry, {"Layer", "Align", "Left"}, QStringLiteral("Left"));
    leaf(registry, {"Layer", "Align", "Horizontal Center"}, QStringLiteral("Horizontal Center"));
    leaf(registry, {"Layer", "Align", "Right"}, QStringLiteral("Right"));
    leaf(registry, {"Layer", "Align Layers To Selection", "Top"}, QStringLiteral("Top"));
    leaf(registry, {"Layer", "Align Layers To Selection", "Vertical Center"},
         QStringLiteral("Vertical Center"));
    leaf(registry, {"Layer", "Align Layers To Selection", "Bottom"}, QStringLiteral("Bottom"));
    leaf(registry, {"Layer", "Align Layers To Selection", "Left"}, QStringLiteral("Left"));
    leaf(registry, {"Layer", "Align Layers To Selection", "Horizontal Center"},
         QStringLiteral("Horizontal Center"));
    leaf(registry, {"Layer", "Align Layers To Selection", "Right"}, QStringLiteral("Right"));
    leaf(registry, {"Layer", "Distribute", "Top"}, QStringLiteral("Top"));
    leaf(registry, {"Layer", "Distribute", "Vertical Center"}, QStringLiteral("Vertical Center"));
    leaf(registry, {"Layer", "Distribute", "Bottom"}, QStringLiteral("Bottom"));
    leaf(registry, {"Layer", "Distribute", "Left"}, QStringLiteral("Left"));
    leaf(registry, {"Layer", "Distribute", "Horizontal Center"}, QStringLiteral("Horizontal Center"));
    leaf(registry, {"Layer", "Distribute", "Right"}, QStringLiteral("Right"));
    registry.add(command_ids::LayerLockLayersAll, {"Layer", "Lock Layers", "All"},
                 QStringLiteral("All"), QKeySequence(), true);
    registry.add(command_ids::LayerLockLayersTransparency,
                 {"Layer", "Lock Layers", "Transparency"}, QStringLiteral("Transparency"),
                 QKeySequence(), true);
    registry.add(command_ids::LayerLockLayersImage, {"Layer", "Lock Layers", "Image"},
                 QStringLiteral("Image"), QKeySequence(), true);
    registry.add(command_ids::LayerLockLayersPosition, {"Layer", "Lock Layers", "Position"},
                 QStringLiteral("Position"), QKeySequence(), true);
    registry.add(command_ids::LayerLockAllInGroup, {"Layer", "Lock All Layers In Group…"},
                 QStringLiteral("Lock All Layers In Group…"), QKeySequence(), true);
    registry.addSeparator({"Layer"});
    registry.add(command_ids::LayerMergeDown, {"Layer", "Merge Down"},
                 QStringLiteral("Merge Down"), QKeySequence(), true);
    registry.add(command_ids::LayerMergeLayers, {"Layer", "Merge Layers"},
                 QStringLiteral("Merge Layers"), QKeySequence(QStringLiteral("Ctrl+E")), true);
    registry.add(command_ids::LayerMergeVisible, {"Layer", "Merge Visible"},
                 QStringLiteral("Merge Visible"), QKeySequence(QStringLiteral("Shift+Ctrl+E")),
                 true);
    registry.add(command_ids::LayerStampVisible, {"Layer", "Stamp Visible"},
                 QStringLiteral("Stamp Visible"),
                 QKeySequence(QStringLiteral("Ctrl+Alt+Shift+E")), true);
    registry.add(command_ids::LayerStampSelected, {"Layer", "Stamp Selected"},
                 QStringLiteral("Stamp Selected"), QKeySequence(QStringLiteral("Ctrl+Alt+E")),
                 true);
    registry.add(command_ids::LayerFlattenImage, {"Layer", "Flatten Image"},
                 QStringLiteral("Flatten Image"), QKeySequence(), true);
    registry.add(command_ids::LayerMergeClippingMask, {"Layer", "Merge Clipping Mask"},
                 QStringLiteral("Merge Clipping Mask"), QKeySequence(), true);
    registry.addSeparator({"Layer"});
    registry.add(command_ids::LayerMattingDefringe, {"Layer", "Matting", "Defringe…"},
                 QStringLiteral("Defringe…"), QKeySequence(), true);
    registry.add(command_ids::LayerMattingRemoveBlack,
                 {"Layer", "Matting", "Remove Black Matte"},
                 QStringLiteral("Remove Black Matte"), QKeySequence(), true);
    registry.add(command_ids::LayerMattingRemoveWhite,
                 {"Layer", "Matting", "Remove White Matte"},
                 QStringLiteral("Remove White Matte"), QKeySequence(), true);

    // Type. Character/Paragraph have their own ids so both menu paths toggle
    // and reflect one panel without sharing a registry action.
    registry.add(CommandSpec{command_ids::TypePanelsCharacter, {"Type", "Panels", "Character"},
                             QStringLiteral("Character"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::TypePanelsParagraph, {"Type", "Panels", "Paragraph"},
                             QStringLiteral("Paragraph"), QKeySequence(), true, true});
    leaf(registry, {"Type", "Panels", "Character Styles"}, QStringLiteral("Character Styles"));
    registry.add(CommandSpec{command_ids::TypePanelsParagraphStyles,
                             {"Type", "Panels", "Paragraph Styles"},
                             QStringLiteral("Paragraph Styles"), QKeySequence(), true, true});
    leaf(registry, {"Type", "Anti-Alias", "None"}, QStringLiteral("None"));
    leaf(registry, {"Type", "Anti-Alias", "Sharp"}, QStringLiteral("Sharp"));
    leaf(registry, {"Type", "Anti-Alias", "Crisp"}, QStringLiteral("Crisp"));
    leaf(registry, {"Type", "Anti-Alias", "Strong"}, QStringLiteral("Strong"));
    leaf(registry, {"Type", "Anti-Alias", "Smooth"}, QStringLiteral("Smooth"));
    leaf(registry, {"Type", "Orientation", "Horizontal"}, QStringLiteral("Horizontal"));
    leaf(registry, {"Type", "Orientation", "Vertical"}, QStringLiteral("Vertical"));
    leaf(registry, {"Type", "Convert To Point Text"}, QStringLiteral("Convert To Point Text"));
    leaf(registry, {"Type", "Convert To Paragraph Text"},
         QStringLiteral("Convert To Paragraph Text"));
    leaf(registry, {"Type", "Warp Text…"}, QStringLiteral("Warp Text…"));
    leaf(registry, {"Type", "Rasterize Type Layer"}, QStringLiteral("Rasterize Type Layer"));
    leaf(registry, {"Type", "Create Work Path"}, QStringLiteral("Create Work Path"));
    leaf(registry, {"Type", "Convert to Shape"}, QStringLiteral("Convert to Shape"));
    leaf(registry, {"Type", "Font Preview Size", "None"}, QStringLiteral("None"));
    leaf(registry, {"Type", "Font Preview Size", "Small"}, QStringLiteral("Small"));
    leaf(registry, {"Type", "Font Preview Size", "Medium"}, QStringLiteral("Medium"));
    leaf(registry, {"Type", "Font Preview Size", "Large"}, QStringLiteral("Large"));
    leaf(registry, {"Type", "Font Preview Size", "Extra Large"}, QStringLiteral("Extra Large"));
    leaf(registry, {"Type", "Font Preview Size", "Huge"}, QStringLiteral("Huge"));
    leaf(registry, {"Type", "Language Options", "Middle Eastern features"},
         QStringLiteral("Middle Eastern features"));
    leaf(registry, {"Type", "Language Options", "East Asian features"},
         QStringLiteral("East Asian features"));
    leaf(registry, {"Type", "Update All Text Layers"}, QStringLiteral("Update All Text Layers"));
    leaf(registry, {"Type", "Check Spelling…"}, QStringLiteral("Check Spelling…"));
    leaf(registry, {"Type", "Find And Replace Text…"}, QStringLiteral("Find And Replace Text…"));
    leaf(registry, {"Type", "Paste Lorem Ipsum"}, QStringLiteral("Paste Lorem Ipsum"));
    leaf(registry, {"Type", "Load Default Type Styles"}, QStringLiteral("Load Default Type Styles"));
    leaf(registry, {"Type", "Save Default Type Styles"}, QStringLiteral("Save Default Type Styles"));
    leaf(registry, {"Type", "Extrude to 3D"}, QStringLiteral("Extrude to 3D"));

    // Select
    registry.add(command_ids::SelectAll, {"Select", "All"}, QStringLiteral("All"),
                 QKeySequence(QStringLiteral("Ctrl+A")), true);
    registry.add(command_ids::SelectDeselect, {"Select", "Deselect"}, QStringLiteral("Deselect"),
                 QKeySequence(QStringLiteral("Ctrl+D")), true);
    registry.addSeparator({"Select"});
    registry.add(command_ids::SelectReselect, {"Select", "Reselect"}, QStringLiteral("Reselect"),
                 QKeySequence(QStringLiteral("Shift+Ctrl+D")), true);
    registry.add(command_ids::SelectInverse, {"Select", "Inverse"}, QStringLiteral("Inverse"),
                 QKeySequence(QStringLiteral("Shift+Ctrl+I")), true);
    registry.addSeparator({"Select"});
    registry.add(command_ids::SelectAllLayers, {"Select", "All Layers"},
                 QStringLiteral("All Layers"), QKeySequence(QStringLiteral("Ctrl+Alt+A")), true);
    registry.add(command_ids::SelectDeselectLayers, {"Select", "Deselect Layers"},
                 QStringLiteral("Deselect Layers"), QKeySequence(), true);
    registry.add(command_ids::SelectSimilarLayers, {"Select", "Similar Layers"},
                 QStringLiteral("Similar Layers"), QKeySequence(), true);
    registry.addSeparator({"Select"});
    leaf(registry, {"Select", "Color Range…"}, QStringLiteral("Color Range…"));
    leaf(registry, {"Select", "Refine Edge…"}, QStringLiteral("Refine Edge…"),
         QStringLiteral("Ctrl+Alt+R"));
    registry.addSeparator({"Select"});
    registry.add(command_ids::SelectModifyBorder, {"Select", "Modify", "Border…"},
                 QStringLiteral("Border…"), QKeySequence(), true);
    registry.add(command_ids::SelectModifySmooth, {"Select", "Modify", "Smooth…"},
                 QStringLiteral("Smooth…"), QKeySequence(), true);
    registry.add(command_ids::SelectModifyExpand, {"Select", "Modify", "Expand…"},
                 QStringLiteral("Expand…"), QKeySequence(), true);
    registry.add(command_ids::SelectModifyContract, {"Select", "Modify", "Contract…"},
                 QStringLiteral("Contract…"), QKeySequence(), true);
    registry.add(command_ids::SelectModifyFeather, {"Select", "Modify", "Feather…"},
                 QStringLiteral("Feather…"), QKeySequence(QStringLiteral("Shift+F6")), true);
    registry.addSeparator({"Select"});
    registry.add(command_ids::SelectGrow, {"Select", "Grow"}, QStringLiteral("Grow"),
                 QKeySequence(), true);
    registry.add(command_ids::SelectSimilar, {"Select", "Similar"}, QStringLiteral("Similar"),
                 QKeySequence(), true);
    registry.addSeparator({"Select"});
    leaf(registry, {"Select", "Transform Selection"}, QStringLiteral("Transform Selection"));
    registry.add(command_ids::SelectSave, {"Select", "Save Selection…"},
                 QStringLiteral("Save Selection…"), QKeySequence(), true);
    registry.add(command_ids::SelectLoad, {"Select", "Load Selection…"},
                 QStringLiteral("Load Selection…"), QKeySequence(), true);

    // Filter
    registry.add(command_ids::FilterLastFilter, {"Filter", "Last Filter"},
                 QStringLiteral("Last Filter"), QKeySequence(QStringLiteral("Ctrl+F")), true);
    registry.add(command_ids::FilterLastFilterSettings, {"Filter", "Last Filter Settings…"},
                 QStringLiteral("Last Filter Settings…"),
                 QKeySequence(QStringLiteral("Ctrl+Alt+F")), true);
    registry.add(command_ids::FilterConvertForSmartFilters,
                 {"Filter", "Convert for Smart Filters"},
                 QStringLiteral("Convert for Smart Filters"), QKeySequence(), true);
    registry.addSeparator({"Filter"});
    leaf(registry, {"Filter", "Filter Gallery…"}, QStringLiteral("Filter Gallery…"));
    leaf(registry, {"Filter", "Adaptive Wide Angle…"}, QStringLiteral("Adaptive Wide Angle…"));
    registry.add(command_ids::FilterPicturaRaw, {"Filter", "Pictura Raw…"},
                 QStringLiteral("Pictura Raw…"), QKeySequence(), true);
    leaf(registry, {"Filter", "Lens Correction…"}, QStringLiteral("Lens Correction…"));
    leaf(registry, {"Filter", "Liquify…"}, QStringLiteral("Liquify…"), QStringLiteral("Shift+Ctrl+X"));
    leaf(registry, {"Filter", "Oil Paint…"}, QStringLiteral("Oil Paint…"));
    leaf(registry, {"Filter", "Vanishing Point…"}, QStringLiteral("Vanishing Point…"),
         QStringLiteral("Alt+Ctrl+V"));
    registry.addSeparator({"Filter"});
    leaf(registry, {"Filter", "Artistic", "Colored Pencil"}, QStringLiteral("Colored Pencil"));
    leaf(registry, {"Filter", "Artistic", "Cutout"}, QStringLiteral("Cutout"));
    leaf(registry, {"Filter", "Artistic", "Dry Brush"}, QStringLiteral("Dry Brush"));
    leaf(registry, {"Filter", "Artistic", "Film Grain"}, QStringLiteral("Film Grain"));
    leaf(registry, {"Filter", "Artistic", "Fresco"}, QStringLiteral("Fresco"));
    leaf(registry, {"Filter", "Artistic", "Neon Glow"}, QStringLiteral("Neon Glow"));
    leaf(registry, {"Filter", "Artistic", "Paint Daubs"}, QStringLiteral("Paint Daubs"));
    leaf(registry, {"Filter", "Artistic", "Palette Knife"}, QStringLiteral("Palette Knife"));
    leaf(registry, {"Filter", "Artistic", "Plastic Wrap"}, QStringLiteral("Plastic Wrap"));
    leaf(registry, {"Filter", "Artistic", "Poster Edges"}, QStringLiteral("Poster Edges"));
    leaf(registry, {"Filter", "Artistic", "Rough Pastels"}, QStringLiteral("Rough Pastels"));
    leaf(registry, {"Filter", "Artistic", "Smudge Stick"}, QStringLiteral("Smudge Stick"));
    leaf(registry, {"Filter", "Artistic", "Sponge"}, QStringLiteral("Sponge"));
    leaf(registry, {"Filter", "Artistic", "Underpainting"}, QStringLiteral("Underpainting"));
    leaf(registry, {"Filter", "Artistic", "Watercolor"}, QStringLiteral("Watercolor"));
    leaf(registry, {"Filter", "Blur", "Field Blur"}, QStringLiteral("Field Blur"));
    leaf(registry, {"Filter", "Blur", "Iris Blur"}, QStringLiteral("Iris Blur"));
    leaf(registry, {"Filter", "Blur", "Tilt-Shift"}, QStringLiteral("Tilt-Shift"));
    leaf(registry, {"Filter", "Blur", "Gaussian Blur"}, QStringLiteral("Gaussian Blur"));
    leaf(registry, {"Filter", "Blur", "Motion Blur"}, QStringLiteral("Motion Blur"));
    leaf(registry, {"Filter", "Blur", "Radial Blur"}, QStringLiteral("Radial Blur"));
    leaf(registry, {"Filter", "Blur", "Box Blur"}, QStringLiteral("Box Blur"));
    leaf(registry, {"Filter", "Blur", "Shape Blur"}, QStringLiteral("Shape Blur"));
    leaf(registry, {"Filter", "Blur", "Surface Blur"}, QStringLiteral("Surface Blur"));
    leaf(registry, {"Filter", "Blur", "Lens Blur"}, QStringLiteral("Lens Blur"));
    leaf(registry, {"Filter", "Blur", "Smart Blur"}, QStringLiteral("Smart Blur"));
    leaf(registry, {"Filter", "Blur", "Average"}, QStringLiteral("Average"));
    leaf(registry, {"Filter", "Blur", "Blur"}, QStringLiteral("Blur"));
    leaf(registry, {"Filter", "Blur", "Blur More"}, QStringLiteral("Blur More"));
    leaf(registry, {"Filter", "Brush Strokes", "Accented Edges"}, QStringLiteral("Accented Edges"));
    leaf(registry, {"Filter", "Brush Strokes", "Angled Strokes"}, QStringLiteral("Angled Strokes"));
    leaf(registry, {"Filter", "Brush Strokes", "Crosshatch"}, QStringLiteral("Crosshatch"));
    leaf(registry, {"Filter", "Brush Strokes", "Dark Strokes"}, QStringLiteral("Dark Strokes"));
    leaf(registry, {"Filter", "Brush Strokes", "Ink Outlines"}, QStringLiteral("Ink Outlines"));
    leaf(registry, {"Filter", "Brush Strokes", "Spatter"}, QStringLiteral("Spatter"));
    leaf(registry, {"Filter", "Brush Strokes", "Sprayed Strokes"}, QStringLiteral("Sprayed Strokes"));
    leaf(registry, {"Filter", "Brush Strokes", "Sumi-e"}, QStringLiteral("Sumi-e"));
    leaf(registry, {"Filter", "Distort", "Displace"}, QStringLiteral("Displace"));
    leaf(registry, {"Filter", "Distort", "Glass"}, QStringLiteral("Glass"));
    leaf(registry, {"Filter", "Distort", "Ocean Ripple"}, QStringLiteral("Ocean Ripple"));
    leaf(registry, {"Filter", "Distort", "Diffuse Glow"}, QStringLiteral("Diffuse Glow"));
    leaf(registry, {"Filter", "Distort", "Pinch"}, QStringLiteral("Pinch"));
    leaf(registry, {"Filter", "Distort", "Polar Coordinates"}, QStringLiteral("Polar Coordinates"));
    leaf(registry, {"Filter", "Distort", "Ripple"}, QStringLiteral("Ripple"));
    leaf(registry, {"Filter", "Distort", "Shear"}, QStringLiteral("Shear"));
    leaf(registry, {"Filter", "Distort", "Spherize"}, QStringLiteral("Spherize"));
    leaf(registry, {"Filter", "Distort", "Twirl"}, QStringLiteral("Twirl"));
    leaf(registry, {"Filter", "Distort", "Wave"}, QStringLiteral("Wave"));
    leaf(registry, {"Filter", "Distort", "ZigZag"}, QStringLiteral("ZigZag"));
    leaf(registry, {"Filter", "Noise", "Add Noise"}, QStringLiteral("Add Noise"));
    leaf(registry, {"Filter", "Noise", "Despeckle"}, QStringLiteral("Despeckle"));
    leaf(registry, {"Filter", "Noise", "Dust & Scratches"}, QStringLiteral("Dust & Scratches"));
    leaf(registry, {"Filter", "Noise", "Median"}, QStringLiteral("Median"));
    leaf(registry, {"Filter", "Noise", "Reduce Noise"}, QStringLiteral("Reduce Noise"));
    leaf(registry, {"Filter", "Pixelate", "Color Halftone"}, QStringLiteral("Color Halftone"));
    leaf(registry, {"Filter", "Pixelate", "Crystallize"}, QStringLiteral("Crystallize"));
    leaf(registry, {"Filter", "Pixelate", "Facet"}, QStringLiteral("Facet"));
    leaf(registry, {"Filter", "Pixelate", "Fragment"}, QStringLiteral("Fragment"));
    leaf(registry, {"Filter", "Pixelate", "Mezzotint"}, QStringLiteral("Mezzotint"));
    leaf(registry, {"Filter", "Pixelate", "Mosaic"}, QStringLiteral("Mosaic"));
    leaf(registry, {"Filter", "Pixelate", "Pointillize"}, QStringLiteral("Pointillize"));
    leaf(registry, {"Filter", "Render", "Clouds"}, QStringLiteral("Clouds"));
    leaf(registry, {"Filter", "Render", "Difference Clouds"}, QStringLiteral("Difference Clouds"));
    leaf(registry, {"Filter", "Render", "Fibers"}, QStringLiteral("Fibers"));
    leaf(registry, {"Filter", "Render", "Lens Flare"}, QStringLiteral("Lens Flare"));
    leaf(registry, {"Filter", "Render", "Lighting Effects"}, QStringLiteral("Lighting Effects"));
    leaf(registry, {"Filter", "Sharpen", "Smart Sharpen"}, QStringLiteral("Smart Sharpen"));
    leaf(registry, {"Filter", "Sharpen", "Unsharp Mask"}, QStringLiteral("Unsharp Mask"));
    leaf(registry, {"Filter", "Sharpen", "Sharpen"}, QStringLiteral("Sharpen"));
    leaf(registry, {"Filter", "Sharpen", "Sharpen Edges"}, QStringLiteral("Sharpen Edges"));
    leaf(registry, {"Filter", "Sharpen", "Sharpen More"}, QStringLiteral("Sharpen More"));
    leaf(registry, {"Filter", "Sketch", "Bas Relief"}, QStringLiteral("Bas Relief"));
    leaf(registry, {"Filter", "Sketch", "Chalk & Charcoal"}, QStringLiteral("Chalk & Charcoal"));
    leaf(registry, {"Filter", "Sketch", "Charcoal"}, QStringLiteral("Charcoal"));
    leaf(registry, {"Filter", "Sketch", "Chrome"}, QStringLiteral("Chrome"));
    leaf(registry, {"Filter", "Sketch", "Conté Crayon"}, QStringLiteral("Conté Crayon"));
    leaf(registry, {"Filter", "Sketch", "Graphic Pen"}, QStringLiteral("Graphic Pen"));
    leaf(registry, {"Filter", "Sketch", "Halftone Pattern"}, QStringLiteral("Halftone Pattern"));
    leaf(registry, {"Filter", "Sketch", "Note Paper"}, QStringLiteral("Note Paper"));
    leaf(registry, {"Filter", "Sketch", "Photocopy"}, QStringLiteral("Photocopy"));
    leaf(registry, {"Filter", "Sketch", "Plaster"}, QStringLiteral("Plaster"));
    leaf(registry, {"Filter", "Sketch", "Reticulation"}, QStringLiteral("Reticulation"));
    leaf(registry, {"Filter", "Sketch", "Stamp"}, QStringLiteral("Stamp"));
    leaf(registry, {"Filter", "Sketch", "Torn Edges"}, QStringLiteral("Torn Edges"));
    leaf(registry, {"Filter", "Sketch", "Water Paper"}, QStringLiteral("Water Paper"));
    leaf(registry, {"Filter", "Stylize", "Diffuse"}, QStringLiteral("Diffuse"));
    leaf(registry, {"Filter", "Stylize", "Emboss"}, QStringLiteral("Emboss"));
    leaf(registry, {"Filter", "Stylize", "Extrude"}, QStringLiteral("Extrude"));
    leaf(registry, {"Filter", "Stylize", "Find Edges"}, QStringLiteral("Find Edges"));
    leaf(registry, {"Filter", "Stylize", "Glowing Edges"}, QStringLiteral("Glowing Edges"));
    leaf(registry, {"Filter", "Stylize", "Solarize"}, QStringLiteral("Solarize"));
    leaf(registry, {"Filter", "Stylize", "Tiles"}, QStringLiteral("Tiles"));
    leaf(registry, {"Filter", "Stylize", "Trace Contour"}, QStringLiteral("Trace Contour"));
    leaf(registry, {"Filter", "Stylize", "Wind"}, QStringLiteral("Wind"));
    leaf(registry, {"Filter", "Texture", "Craquelure"}, QStringLiteral("Craquelure"));
    leaf(registry, {"Filter", "Texture", "Grain"}, QStringLiteral("Grain"));
    leaf(registry, {"Filter", "Texture", "Mosaic Tiles"}, QStringLiteral("Mosaic Tiles"));
    leaf(registry, {"Filter", "Texture", "Patchwork"}, QStringLiteral("Patchwork"));
    leaf(registry, {"Filter", "Texture", "Stained Glass"}, QStringLiteral("Stained Glass"));
    leaf(registry, {"Filter", "Texture", "Texturizer"}, QStringLiteral("Texturizer"));
    leaf(registry, {"Filter", "Video", "De-Interlace"}, QStringLiteral("De-Interlace"));
    leaf(registry, {"Filter", "Video", "NTSC Colors"}, QStringLiteral("NTSC Colors"));
    leaf(registry, {"Filter", "Other", "Custom"}, QStringLiteral("Custom"));
    leaf(registry, {"Filter", "Other", "High Pass"}, QStringLiteral("High Pass"));
    leaf(registry, {"Filter", "Other", "Maximum"}, QStringLiteral("Maximum"));
    leaf(registry, {"Filter", "Other", "Minimum"}, QStringLiteral("Minimum"));
    leaf(registry, {"Filter", "Other", "Offset"}, QStringLiteral("Offset"));
    leaf(registry, {"Filter", "Digimarc", "Embed Watermark"}, QStringLiteral("Embed Watermark"));
    leaf(registry, {"Filter", "Digimarc", "Read Watermark"}, QStringLiteral("Read Watermark"));
    registry.addSeparator({"Filter"});
    leaf(registry, {"Filter", "Extract…"}, QStringLiteral("Extract…"));
    leaf(registry, {"Filter", "Pattern Maker…"}, QStringLiteral("Pattern Maker…"));
    registry.addSeparator({"Filter"});
    leaf(registry, {"Filter", "Browse Filters Online…"}, QStringLiteral("Browse Filters Online…"));

    // View
    leaf(registry, {"View", "Proof Setup", "Working CMYK"}, QStringLiteral("Working CMYK"));
    leaf(registry, {"View", "Proof Setup", "Working Gray"}, QStringLiteral("Working Gray"));
    leaf(registry, {"View", "Proof Setup", "Custom"}, QStringLiteral("Custom"));
    leaf(registry, {"View", "Proof Setup", "Color Blindness", "Protanopia-type"},
         QStringLiteral("Protanopia-type"));
    leaf(registry, {"View", "Proof Setup", "Color Blindness", "Deuteranopia-type"},
         QStringLiteral("Deuteranopia-type"));
    leaf(registry, {"View", "Proof Colors"}, QStringLiteral("Proof Colors"), QStringLiteral("Ctrl+Y"));
    leaf(registry, {"View", "Gamut Warning"}, QStringLiteral("Gamut Warning"),
         QStringLiteral("Shift+Ctrl+Y"));
    registry.addSeparator({"View"});
    registry.add(command_ids::ViewZoomIn, {"View", "Zoom In"}, QStringLiteral("Zoom In"),
                 QKeySequence(QStringLiteral("Ctrl++")), true);
    registry.add(command_ids::ViewZoomOut, {"View", "Zoom Out"}, QStringLiteral("Zoom Out"),
                 QKeySequence(QStringLiteral("Ctrl+-")), true);
    registry.add(command_ids::ViewFitOnScreen, {"View", "Fit On Screen"},
                 QStringLiteral("Fit On Screen"), QKeySequence(QStringLiteral("Ctrl+0")), true);
    registry.add(command_ids::ViewActualPixels, {"View", "100%"}, QStringLiteral("100%"),
                 QKeySequence(QStringLiteral("Ctrl+1")), true);
    leaf(registry, {"View", "200%"}, QStringLiteral("200%"));
    leaf(registry, {"View", "Print Size"}, QStringLiteral("Print Size"));
    registry.addSeparator({"View"});
    registry.add(CommandSpec{command_ids::ViewScreenModeStandard,
                             {"View", "Screen Mode", "Standard Screen Mode"},
                             QStringLiteral("Standard Screen Mode"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::ViewScreenModeFullWithMenuBar,
                             {"View", "Screen Mode", "Full Screen Mode With Menu Bar"},
                             QStringLiteral("Full Screen Mode With Menu Bar"), QKeySequence(), true,
                             true});
    registry.add(CommandSpec{command_ids::ViewScreenModeFull,
                             {"View", "Screen Mode", "Full Screen Mode"},
                             QStringLiteral("Full Screen Mode"), QKeySequence(), true, true});
    registry.addSeparator({"View"});
    registry.add(CommandSpec{command_ids::ViewRulers, {"View", "Rulers"}, QStringLiteral("Rulers"),
                             QKeySequence(QStringLiteral("Ctrl+R")), true, true});
    leaf(registry, {"View", "Snap"}, QStringLiteral("Snap"), QStringLiteral("Shift+Ctrl+;"));
    leaf(registry, {"View", "Snap To", "Guides"}, QStringLiteral("Guides"));
    leaf(registry, {"View", "Snap To", "Grid"}, QStringLiteral("Grid"));
    leaf(registry, {"View", "Snap To", "Layers"}, QStringLiteral("Layers"));
    leaf(registry, {"View", "Snap To", "Slices"}, QStringLiteral("Slices"));
    leaf(registry, {"View", "Snap To", "Document Bounds"}, QStringLiteral("Document Bounds"));
    leaf(registry, {"View", "Snap To", "All"}, QStringLiteral("All"));
    leaf(registry, {"View", "Snap To", "None"}, QStringLiteral("None"));
    registry.add(CommandSpec{command_ids::ViewLockGuides, {"View", "Lock Guides"},
                             QStringLiteral("Lock Guides"),
                             QKeySequence(QStringLiteral("Alt+Ctrl+;")), true, true});
    registry.add(command_ids::ViewClearGuides, {"View", "Clear Guides"},
                 QStringLiteral("Clear Guides"), QKeySequence(), true);
    registry.add(command_ids::ViewNewGuide, {"View", "New Guide…"}, QStringLiteral("New Guide…"),
                 QKeySequence(), true);
    registry.addSeparator({"View"});
    leaf(registry, {"View", "Lock Slices"}, QStringLiteral("Lock Slices"));
    registry.addSeparator({"View"});
    registry.add(CommandSpec{command_ids::ViewShowSelectionEdges,
                             {"View", "Show", "Selection Edges"},
                             QStringLiteral("Selection Edges"), QKeySequence(), true, true});
    leaf(registry, {"View", "Show", "Target Path"}, QStringLiteral("Target Path"));
    leaf(registry, {"View", "Show", "Layer Edges"}, QStringLiteral("Layer Edges"));
    leaf(registry, {"View", "Show", "3D Axis"}, QStringLiteral("3D Axis"));
    registry.add(CommandSpec{command_ids::ViewShowGuides, {"View", "Show", "Guides"},
                             QStringLiteral("Guides"), QKeySequence(QStringLiteral("Ctrl+;")),
                             true, true});
    leaf(registry, {"View", "Show", "Grid"}, QStringLiteral("Grid"));
    leaf(registry, {"View", "Show", "Count"}, QStringLiteral("Count"));
    leaf(registry, {"View", "Show", "Slices"}, QStringLiteral("Slices"));
    leaf(registry, {"View", "Show", "Notes"}, QStringLiteral("Notes"));
    leaf(registry, {"View", "Show", "Pixel Grid"}, QStringLiteral("Pixel Grid"));
    leaf(registry, {"View", "Show", "All"}, QStringLiteral("All"));
    leaf(registry, {"View", "Show", "None"}, QStringLiteral("None"));
    leaf(registry, {"View", "Extras"}, QStringLiteral("Extras"), QStringLiteral("Ctrl+H"));
    registry.addSeparator({"View"});
    leaf(registry, {"View", "Align"}, QStringLiteral("Align"));
    leaf(registry, {"View", "Align To"}, QStringLiteral("Align To"));
    registry.addSeparator({"View"});
    registry.add(CommandSpec{command_ids::ViewToolHints, {"View", "Tool Hints"},
                             QStringLiteral("Tool Hints"), QKeySequence(), true, true});

    // Window
    leaf(registry, {"Window", "Arrange", "Cascade"}, QStringLiteral("Cascade"));
    leaf(registry, {"Window", "Arrange", "Tile"}, QStringLiteral("Tile"));
    leaf(registry, {"Window", "Arrange", "Consolidate All to Tabs"},
         QStringLiteral("Consolidate All to Tabs"));
    leaf(registry, {"Window", "Arrange", "Float in Window"}, QStringLiteral("Float in Window"));
    leaf(registry, {"Window", "Arrange", "Float All in Windows"},
         QStringLiteral("Float All in Windows"));
    leaf(registry, {"Window", "Arrange", "Match Zoom"}, QStringLiteral("Match Zoom"));
    leaf(registry, {"Window", "Arrange", "Match Location"}, QStringLiteral("Match Location"));
    leaf(registry, {"Window", "Arrange", "Match All"}, QStringLiteral("Match All"));
    leaf(registry, {"Window", "Arrange", "New Window For [file]"},
         QStringLiteral("New Window For [file]"));
    registry.add(CommandSpec{command_ids::WindowWorkspaceEssentials,
                             {"Window", "Workspace", "Essentials"}, QStringLiteral("Essentials"),
                             QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowWorkspacePainting,
                             {"Window", "Workspace", "Painting"}, QStringLiteral("Painting"),
                             QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowWorkspacePhotography,
                             {"Window", "Workspace", "Photography"}, QStringLiteral("Photography"),
                             QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowWorkspaceTypography,
                             {"Window", "Workspace", "Typography"}, QStringLiteral("Typography"),
                             QKeySequence(), true, true});
    registry.addSeparator({"Window", "Workspace"});
    registry.add(CommandSpec{command_ids::WindowWorkspaceNew,
                             {"Window", "Workspace", "New Workspace…"},
                             QStringLiteral("New Workspace…"), QKeySequence(), true, false});
    registry.add(CommandSpec{command_ids::WindowWorkspaceDelete,
                             {"Window", "Workspace", "Delete Workspace…"},
                             QStringLiteral("Delete Workspace…"), QKeySequence(), true, false});
    registry.add(CommandSpec{command_ids::WindowWorkspaceReset,
                             {"Window", "Workspace", "Reset [Workspace]"},
                             QStringLiteral("Reset [Workspace]"), QKeySequence(), true, false});
    registry.addSeparator({"Window", "Workspace"});
    leaf(registry, {"Window", "Workspace", "Keyboard Shortcuts & Menus…"},
         QStringLiteral("Keyboard Shortcuts & Menus…"));
    registry.addSeparator({"Window"});
    leaf(registry, {"Window", "Panels", "3D"}, QStringLiteral("3D"));
    registry.add(CommandSpec{command_ids::WindowPanelsActions, {"Window", "Panels", "Actions"},
                             QStringLiteral("Actions"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsAdjustments, {"Window", "Panels", "Adjustments"},
                             QStringLiteral("Adjustments"), QKeySequence(), true, true});
    leaf(registry, {"Window", "Panels", "Animation"}, QStringLiteral("Animation"));
    registry.add(CommandSpec{command_ids::WindowPanelsBrush, {"Window", "Panels", "Brush"},
                             QStringLiteral("Brush"), QKeySequence(Qt::Key_F5), true, true});
    leaf(registry, {"Window", "Panels", "Brush Presets"}, QStringLiteral("Brush Presets"));
    registry.add(CommandSpec{command_ids::WindowPanelsChannels, {"Window", "Panels", "Channels"},
                             QStringLiteral("Channels"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsCharacter, {"Window", "Panels", "Character"},
                             QStringLiteral("Character"), QKeySequence(), true, true});
    leaf(registry, {"Window", "Panels", "Character Styles"}, QStringLiteral("Character Styles"));
    registry.add(CommandSpec{command_ids::WindowPanelsCloneSource,
                             {"Window", "Panels", "Clone Source"}, QStringLiteral("Clone Source"),
                             QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsColor, {"Window", "Panels", "Color"},
                             QStringLiteral("Color"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsGlyphs, {"Window", "Panels", "Glyphs"},
                             QStringLiteral("Glyphs"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsGradients, {"Window", "Panels", "Gradients"},
                             QStringLiteral("Gradients"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsHistogram, {"Window", "Panels", "Histogram"},
                             QStringLiteral("Histogram"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsHistory, {"Window", "Panels", "History"},
                             QStringLiteral("History"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsInfo, {"Window", "Panels", "Info"},
                             QStringLiteral("Info"), QKeySequence(), true, true});
    leaf(registry, {"Window", "Panels", "Layer Comps"}, QStringLiteral("Layer Comps"));
    registry.add(CommandSpec{command_ids::WindowPanelsLayers, {"Window", "Panels", "Layers"},
                             QStringLiteral("Layers"), QKeySequence(), true, true});
    leaf(registry, {"Window", "Panels", "Measurement Log"}, QStringLiteral("Measurement Log"));
    registry.add(CommandSpec{command_ids::WindowPanelsNavigator, {"Window", "Panels", "Navigator"},
                             QStringLiteral("Navigator"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsNotes, {"Window", "Panels", "Notes"},
                             QStringLiteral("Notes"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsParagraph, {"Window", "Panels", "Paragraph"},
                             QStringLiteral("Paragraph"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsParagraphStyles,
                             {"Window", "Panels", "Paragraph Styles"},
                             QStringLiteral("Paragraph Styles"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsPaths, {"Window", "Panels", "Paths"},
                             QStringLiteral("Paths"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsPatterns, {"Window", "Panels", "Patterns"},
                             QStringLiteral("Patterns"), QKeySequence(), true, true});
    registry.add(CommandSpec{command_ids::WindowPanelsProperties, {"Window", "Panels", "Properties"},
                             QStringLiteral("Properties"), QKeySequence(), true, true});
    leaf(registry, {"Window", "Panels", "Styles"}, QStringLiteral("Styles"));
    registry.add(CommandSpec{command_ids::WindowPanelsSwatches, {"Window", "Panels", "Swatches"},
                             QStringLiteral("Swatches"), QKeySequence(), true, true});
    leaf(registry, {"Window", "Panels", "Timeline"}, QStringLiteral("Timeline"));
    leaf(registry, {"Window", "Panels", "Tool Presets"}, QStringLiteral("Tool Presets"));
    registry.add(CommandSpec{command_ids::WindowPanelsTools, {"Window", "Panels", "Tools"},
                             QStringLiteral("Tools"), QKeySequence(), true, true});
    registry.addSeparator({"Window"});
    registry.add(CommandSpec{command_ids::ViewOptions, {"Window", "Options"},
                             QStringLiteral("Options"), QKeySequence(), true, true});

    // Help
    leaf(registry, {"Help", "Kooka Pictura Help"}, QStringLiteral("Kooka Pictura Help"),
         QStringLiteral("F1"));
    leaf(registry, {"Help", "Online Support"}, QStringLiteral("Online Support"));
    leaf(registry, {"Help", "Full Product Family Help"}, QStringLiteral("Full Product Family Help"));
    leaf(registry, {"Help", "Product Improvement Program"},
         QStringLiteral("Product Improvement Program"));
    registry.addSeparator({"Help"});
    leaf(registry, {"Help", "About Plug-in"}, QStringLiteral("About Plug-in"));
    leaf(registry, {"Help", "System Info"}, QStringLiteral("System Info"));
    registry.addSeparator({"Help"});
    registry.add(command_ids::HelpAbout, {"Help", "About Kooka Pictura"},
                 QStringLiteral("About Kooka Pictura"), QKeySequence(), true);
}

} // namespace pictura
