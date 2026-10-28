#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QLabel;

namespace pictura {

// Filter > Digimarc > Read Watermark's result: the decoded in-tree watermark
// (an approximation, not Digimarc Barcode) or "no watermark found".
class DigimarcReadDialog : public QDialog {
    Q_OBJECT

public:
    DigimarcReadDialog(bool found, const QString& kind, int creatorId, int id,
                       QWidget* parent = nullptr);

    // Show the result under runDialog. Returns when the user dismisses it.
    static void showResult(bool found, const QString& kind, int creatorId, int id,
                           QWidget* parent = nullptr);

private:
    QLabel* detail_ = nullptr;
};

} // namespace pictura
