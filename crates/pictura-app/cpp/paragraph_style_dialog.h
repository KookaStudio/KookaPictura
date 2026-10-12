#pragma once

#include <QtGui/QColor>
#include <QtWidgets/QDialog>

#include "api/type_api.h"

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QFontComboBox;
class QLabel;
class QLineEdit;
class QListWidget;
class QSpinBox;
class QStackedWidget;
class QToolButton;

namespace pictura {

// Photoshop's Paragraph Style Options: a list of pages down the left, the
// chosen one on the right, with the style's name above them. All seven CS6
// pages are offered — Basic and Advanced Character Formats, OpenType Features,
// Indents and Spacing, Composition, Justification, and Hyphenation — over the
// extended type model. The style's attributes are the effective ones read back
// from the sheet (type_api::paragraphStyle*), and an unset field reports the
// document default.
//
// Preview is live while editing an existing style: a control change emits
// previewChanged and the panel re-applies the dialog's attributes without
// recording history; Cancel restores the opening values. Creating a style has
// nothing to preview yet, so the checkbox is disabled.
class ParagraphStyleDialog : public QDialog {
    Q_OBJECT

public:
    ParagraphStyleDialog(const QString& name, const QString& family,
                         const CharacterSetting& character, const ParagraphSetting& paragraph,
                         bool previewAvailable, QWidget* parent = nullptr);

    // The edited style, read back off the widgets. Only meaningful once the
    // dialog has been accepted.
    QString styleName() const;
    QString family() const;
    CharacterSetting characterSetting() const;
    ParagraphSetting paragraphSetting() const;

    QListWidget* pageListForTest() const { return pageList_; }
    QCheckBox* previewForTest() const { return preview_; }
    bool previewAvailableForTest() const { return previewAvailable_; }

signals:
    // A control changed while Preview is checked.
    void previewChanged();

private:
    void addPage(const QString& title, QWidget* page);
    QWidget* buildBasicCharacterPage();
    QWidget* buildAdvancedCharacterPage();
    QWidget* buildOpenTypePage();
    QWidget* buildIndentsPage();
    QWidget* buildCompositionPage();
    QWidget* buildJustificationPage();
    QWidget* buildHyphenationPage();
    void refreshStyles(const QString& family, const QString& wanted);
    void refreshSwatch();
    // Re-apply the dialog's values live when the control changes and Preview is
    // checked.
    void watch(QWidget* control);
    void previewChangedIfChecked();

    QListWidget* pageList_ = nullptr;
    QStackedWidget* pages_ = nullptr;
    QLabel* pageTitle_ = nullptr;

    QLineEdit* name_ = nullptr;
    QCheckBox* preview_ = nullptr;

    QFontComboBox* family_ = nullptr;
    QComboBox* fontStyle_ = nullptr;
    QDoubleSpinBox* size_ = nullptr;
    QComboBox* leadingMode_ = nullptr;
    QDoubleSpinBox* leading_ = nullptr;
    QComboBox* kerningMode_ = nullptr;
    QSpinBox* kerning_ = nullptr;
    QDoubleSpinBox* tracking_ = nullptr;
    QComboBox* caseBox_ = nullptr;
    QComboBox* positionBox_ = nullptr;
    QToolButton* colorSwatch_ = nullptr;
    QCheckBox* strikethrough_ = nullptr;
    QCheckBox* underline_ = nullptr;
    QCheckBox* fauxBold_ = nullptr;
    QCheckBox* fauxItalic_ = nullptr;
    QCheckBox* verticalRoman_ = nullptr;

    QDoubleSpinBox* hScale_ = nullptr;
    QDoubleSpinBox* vScale_ = nullptr;
    QDoubleSpinBox* baselineShift_ = nullptr;
    QComboBox* language_ = nullptr;

    QCheckBox* standardLigatures_ = nullptr;
    QCheckBox* contextualAlternates_ = nullptr;
    QCheckBox* discretionaryLigatures_ = nullptr;
    QCheckBox* swash_ = nullptr;
    QCheckBox* oldstyle_ = nullptr;
    QCheckBox* stylisticAlternates_ = nullptr;
    QCheckBox* titlingAlternates_ = nullptr;
    QCheckBox* ornaments_ = nullptr;
    QCheckBox* ordinals_ = nullptr;
    QCheckBox* fractions_ = nullptr;

    QComboBox* alignment_ = nullptr;
    QDoubleSpinBox* startIndent_ = nullptr;
    QDoubleSpinBox* endIndent_ = nullptr;
    QDoubleSpinBox* firstLineIndent_ = nullptr;
    QDoubleSpinBox* spaceBefore_ = nullptr;
    QDoubleSpinBox* spaceAfter_ = nullptr;

    QComboBox* composer_ = nullptr;
    QCheckBox* hanging_ = nullptr;

    QDoubleSpinBox* wordMin_ = nullptr;
    QDoubleSpinBox* wordDesired_ = nullptr;
    QDoubleSpinBox* wordMax_ = nullptr;
    QDoubleSpinBox* letterMin_ = nullptr;
    QDoubleSpinBox* letterDesired_ = nullptr;
    QDoubleSpinBox* letterMax_ = nullptr;
    QDoubleSpinBox* glyphMin_ = nullptr;
    QDoubleSpinBox* glyphDesired_ = nullptr;
    QDoubleSpinBox* glyphMax_ = nullptr;
    QDoubleSpinBox* autoLeading_ = nullptr;

    QCheckBox* hyphenate_ = nullptr;
    QSpinBox* hyphenWordSize_ = nullptr;
    QSpinBox* hyphenPre_ = nullptr;
    QSpinBox* hyphenPost_ = nullptr;
    QSpinBox* hyphenLimit_ = nullptr;
    QDoubleSpinBox* hyphenZone_ = nullptr;
    QCheckBox* hyphenateCaps_ = nullptr;

    // The dialog's opening values: fields without a control (anti-aliasing,
    // fractional widths, the default font style) carry through unchanged.
    CharacterSetting character_;
    ParagraphSetting paragraph_;
    QString initialFamily_;
    QColor color_;
    bool previewAvailable_ = true;
};

} // namespace pictura
