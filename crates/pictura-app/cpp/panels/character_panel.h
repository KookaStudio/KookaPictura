#pragma once

#include <QtCore/QPoint>
#include <QtCore/QSize>
#include <QtCore/QString>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

#include "api/type_api.h"

#include <array>
#include <functional>

class QComboBox;
class QFontComboBox;
class QLabel;
class QMenu;
class QToolButton;

namespace pictura {

class TextSession;

class NumericField;
class PictureView;

// The Character panel (CS6 `Window > Character`, `Type > Panels > Character`):
// a live attribute editor over the active type layer's `CharacterAttrs`, laid
// out as CS6's: the family; icon-labelled size / leading, kerning / tracking,
// vertical / horizontal scale, baseline shift / colour; the eight style
// buttons; the eight OpenType buttons (greyed where the font lacks the
// feature); and the anti-aliasing method. Every numeric field scrubs: drag
// its icon or value left or right. With no type layer active it shows the
// model defaults and disables editing.
//
// ponytail: no font style combo (the model's font style is not applied by the
// renderer, as in the options bar) and no language (#517).
class CharacterPanel : public QWidget {
    Q_OBJECT

public:
    explicit CharacterPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    // The open type session, if any: while new type is being set the panel
    // edits that session instead of the selected layer.
    void setTextSession(std::function<TextSession*()> session);
    void setShowAsianOptions(bool on);
    void updatePanelMenu(QMenu* menu);
    void performPanelMenuAction(const QString& actionId);
    void refresh();

    // The dense form's natural minimum width would raise the panel column's
    // shared floor; the column supplies the width, so report none.
    QSize minimumSizeHint() const override;

    QComboBox* styleFieldForTest() const { return style_; }
    QComboBox* languageFieldForTest() const { return language_; }
    NumericField* tsumeFieldForTest() const { return tsume_; }
    QComboBox* sizeFieldForTest() const { return size_.combo; }
    QComboBox* leadingFieldForTest() const { return leading_.combo; }
    QComboBox* kerningFieldForTest() const { return kerning_.combo; }
    QLabel* kerningIconForTest() const { return kerning_.icon; }
    QComboBox* trackingFieldForTest() const { return tracking_.combo; }
    NumericField* horizontalScaleFieldForTest() const { return horizontalScale_; }
    // Panel order (see `character_icons::style` / `openType`).
    QToolButton* styleButtonForTest(int index) const { return styles_.at(index); }
    QToolButton* openTypeButtonForTest(int index) const { return openType_.at(index); }
    QToolButton* underlineForTest() const { return styles_.at(6); }
    bool editingEnabledForTest() const;
    void commitForTest();

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;
    void changeEvent(QEvent* event) override;

private:
    // An editable preset combo behind a scrub icon. `parse` reads typed or
    // picked text into the shown setting (false rejects it); `show` writes
    // the shown setting back as text; `scrubFrom` is the value a drag starts
    // at; `scrubTo` stores a scrubbed value into the shown setting.
    struct ComboField {
        QLabel* icon = nullptr;
        QComboBox* combo = nullptr;
        double step = 1.0;
        double minimum = 0.0;
        double maximum = 0.0;
        std::function<bool(const QString&)> parse;
        std::function<QString()> show;
        std::function<double()> scrubFrom;
        std::function<void(double)> scrubTo;
    };

    ComboField* comboFor(QObject* watched);
    void buildComboField(ComboField& field, const QString& name, const QString& tip,
                         const QStringList& presets);
    void commitCombo(ComboField& field);
    void syncCombos();
    void updateIcons();
    // `preview` shows the layer's new attributes without a history state.
    void apply(bool preview = false);
    void resetCharacter();
    bool hasTypeLayer() const;
    void updateColorIcon();

    PictureView* view_ = nullptr;
    bool showAsianOptions_ = false;
    std::function<TextSession*()> session_;
    // The attributes the controls show; edits land here, then `apply` writes
    // them to the layer.
    CharacterSetting shown_{};
    QFontComboBox* family_ = nullptr;
    QComboBox* style_ = nullptr;
    QComboBox* language_ = nullptr;
    ComboField size_;
    ComboField leading_;
    ComboField kerning_;
    ComboField tracking_;
    NumericField* verticalScale_ = nullptr;
    NumericField* horizontalScale_ = nullptr;
    NumericField* baselineShift_ = nullptr;
    NumericField* tsume_ = nullptr;
    QLabel* antiAliasIcon_ = nullptr;
    QComboBox* antiAlias_ = nullptr;
    QToolButton* colorButton_ = nullptr;
    std::array<QToolButton*, 8> styles_{};
    std::array<QToolButton*, 8> openType_{};
    QColor color_ = Qt::black;
    // The combo scrub in progress, if any.
    ComboField* scrubbing_ = nullptr;
    bool scrubMoved_ = false;
    QPoint scrubOrigin_;
    double scrubStart_ = 0.0;
};

} // namespace pictura
