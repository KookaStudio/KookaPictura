#pragma once

#include <QtCore/QSize>
#include <QtCore/QString>
#include <QtWidgets/QWidget>

#include <functional>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QFormLayout;
class QMenu;
class QToolButton;

namespace pictura {

class PictureView;
class TextSession;
struct ParagraphSetting;

// The Paragraph panel (CS6 `Window > Paragraph`, `Type > Panels > Paragraph`): a
// live attribute editor over the open type session, or else the active type
// layer's `ParagraphAttrs`. With neither it shows the model defaults and
// disables editing.
class ParagraphPanel : public QWidget {
    Q_OBJECT

public:
    explicit ParagraphPanel(QWidget* parent = nullptr);

    void setView(PictureView* view);
    // The open type session, if any: while one is open the panel edits that
    // session instead of the selected layer.
    void setTextSession(std::function<TextSession*()> session);
    void setMiddleEastern(bool on);
    void updatePanelMenu(QMenu* menu);
    void performPanelMenuAction(const QString& actionId);
    void refresh();

    // The dense form's natural minimum width would raise the panel column's
    // shared floor; the column supplies the width, so report none.
    QSize minimumSizeHint() const override;

    QComboBox* justifyFieldForTest() const { return justify_; }
    QToolButton* directionLtrForTest() const { return ltr_; }
    QToolButton* directionRtlForTest() const { return rtl_; }
    bool directionVisibleForTest() const;
    QDoubleSpinBox* firstLineIndentFieldForTest() const { return firstLineIndent_; }
    bool editingEnabledForTest() const;
    void commitForTest();

private:
    void apply();
    void openJustification();
    void resetParagraph();
    bool hasTypeLayer() const;
    // The session being edited, null with none open.
    TextSession* openSession() const;
    void readFields(ParagraphSetting& p) const;

    PictureView* view_ = nullptr;
    std::function<TextSession*()> session_;
    bool middleEastern_ = false;
    QFormLayout* form_ = nullptr;
    QWidget* direction_ = nullptr;
    QComboBox* justify_ = nullptr;
    QToolButton* ltr_ = nullptr;
    QToolButton* rtl_ = nullptr;
    QDoubleSpinBox* startIndent_ = nullptr;
    QDoubleSpinBox* endIndent_ = nullptr;
    QDoubleSpinBox* firstLineIndent_ = nullptr;
    QDoubleSpinBox* spaceBefore_ = nullptr;
    QDoubleSpinBox* spaceAfter_ = nullptr;
    QCheckBox* hanging_ = nullptr;
    QCheckBox* hyphenate_ = nullptr;
    QComboBox* composer_ = nullptr;
};

} // namespace pictura
