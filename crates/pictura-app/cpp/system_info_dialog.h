#pragma once

#include "system_info.h"

#include <QtWidgets/QDialog>

class QPlainTextEdit;

namespace pictura {

// Help > System Info: a modeless, read-only environment report with a Copy
// button. The report is a snapshot; `setInfo` replaces it.
class SystemInfoDialog : public QDialog {
    Q_OBJECT

public:
    explicit SystemInfoDialog(const SystemInfo& info, QWidget* parent = nullptr);

    void setInfo(const SystemInfo& info);
    QString reportForTest() const;

private:
    QString report_;
    QPlainTextEdit* text_ = nullptr;
};

} // namespace pictura
