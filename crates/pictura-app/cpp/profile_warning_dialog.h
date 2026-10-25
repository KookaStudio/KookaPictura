#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QRadioButton;

namespace pictura {

// The open-time profile decision dialog. The mismatch case offers the three
// CS6 choices; the missing case offers assign / assign-and-convert / leave
// untagged. Warnings are off by default in Color Settings.
class ProfileWarningDialog : public QDialog {
    Q_OBJECT

public:
    enum Kind { Mismatch, Missing };
    enum Choice {
        UseEmbedded,
        ConvertToWorking,
        DiscardEmbedded,
        AssignWorking,
        AssignThenConvert,
        LeaveUntagged,
    };

    ProfileWarningDialog(Kind kind, const QString& profileName, QWidget* parent = nullptr);

    Choice choice() const;
    void setChoiceForTest(Choice choice);

private:
    Kind kind_;
    QRadioButton* first_ = nullptr;
    QRadioButton* second_ = nullptr;
    QRadioButton* third_ = nullptr;
};

} // namespace pictura
