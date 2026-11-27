#pragma once

#include "preset_store.h"
#include <QtGui/QColor>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QSpinBox>

class QToolButton;

namespace pictura {

// Photoshop's Edit > Fill: Contents (foreground / background / Color / Black /
// 50% Gray / White / Pattern), a Blending group (Mode, Opacity), and Preserve
// Transparency. Ported from photorust's FillDialog.
//
// CS6's Content-Aware and History sources are shown disabled: the engine has no
// Content-Aware fill or history-state brush for this path. The Mode list is the
// Brush modes the fill engine honours (Normal, Dissolve, Behind, Clear).
class FillDialog : public QDialog {
    Q_OBJECT

public:
    explicit FillDialog(const QColor& foreground, const QColor& background,
                        QWidget* parent = nullptr);

    QColor fillColor() const;
    // One of "normal", "dissolve", "behind", "clear".
    QString blendMode() const;
    int opacity() const;
    bool preserveTransparency() const;
    bool isPatternFill() const;
    int selectedPatternIndex() const;
    QByteArray selectedPatternBytes() const;
    int script() const;

private:
    void onContentsChanged(int index);
    void buildPatternGrid();
    void chooseColor();

    QComboBox* contents_ = nullptr;
    QComboBox* mode_ = nullptr;
    QSpinBox* opacity_ = nullptr;
    QCheckBox* preserveTransparency_ = nullptr;
    QToolButton* patternSwatch_ = nullptr;
    QWidget* patternPopup_ = nullptr;
    QCheckBox* scripted_ = nullptr;
    QComboBox* script_ = nullptr;
    PresetRows patterns_;

    QColor foreground_;
    QColor background_;
    QColor customColor_;
    int selectedPattern_ = 0;
};

} // namespace pictura
