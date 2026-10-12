#pragma once

#include <QtGui/QFont>
#include <QtWidgets/QWidget>

#include "tool_options.h"

class QComboBox;
class QFontComboBox;
class QLabel;
class QListWidget;
class QListWidgetItem;
class QSlider;

namespace pictura {

class ToolController;

// Window > Glyphs: every character a font has in a set of Unicode blocks, in a
// grid; a double-click inserts one at the open type edit's caret. Not a CS6
// panel (Photoshop gained it in CC 2015); ported from photorust's GlyphsPanel
// on request (#65).
//
// Qt exposes no character map, so the grid asks the font about each code point
// of the listed blocks: a font whose coverage lies outside them shows fewer
// glyphs than it has.
class GlyphsPanel : public QWidget {
    Q_OBJECT

public:
    explicit GlyphsPanel(QWidget* parent = nullptr);

    // Follow the type tools' font until a family is picked here.
    void setController(ToolController* controller);
    // Insert `item`'s glyph as a double-click does (also the Qt Test's entry).
    void chooseGlyph(QListWidgetItem* item);

protected:
    void showEvent(QShowEvent* event) override;

private:
    void followTypeFont();
    void rebuildStyles();
    // Rebuild the grid now when shown, else on the next show.
    void rebuildGlyphs();
    QFont glyphFont() const;

    ToolController* controller_ = nullptr;
    TypeGroup* type_ = nullptr;
    QFontComboBox* family_ = nullptr;
    QComboBox* style_ = nullptr;
    QComboBox* subset_ = nullptr;
    QListWidget* grid_ = nullptr;
    QSlider* zoom_ = nullptr;
    QLabel* hint_ = nullptr;
    bool familyChosenHere_ = false;
    bool following_ = false;
    bool stale_ = true;
};

} // namespace pictura
