#pragma once

#include <QtWidgets/QDialog>

#include <array>

#include "api/type_api.h"

class QDoubleSpinBox;

namespace pictura {

// The Paragraph panel menu's Justification dialog: word, letter, and glyph
// spacing as Minimum/Desired/Maximum percentages, plus Auto Leading. The
// panel applies the returned setting to the active type layer as one state.
class JustificationDialog : public QDialog {
    Q_OBJECT

public:
    explicit JustificationDialog(const ParagraphSetting& setting, QWidget* parent = nullptr);

    ParagraphSetting setting() const;
    QDoubleSpinBox* fieldForTest(int index) const { return fields_.at(std::size_t(index)); }

private:
    ParagraphSetting setting_;
    // Word, letter, and glyph Min/Desired/Max, then Auto Leading.
    std::array<QDoubleSpinBox*, 10> fields_{};
};

} // namespace pictura
