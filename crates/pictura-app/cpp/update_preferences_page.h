#pragma once

#include <QtWidgets/QWidget>

class QCheckBox;
class QLabel;
class QPushButton;

namespace pictura {

class UpdateChecker;

// Edit > Preferences > Updates. Shows the automatic-check setting, the running
// version, the detected package method, a manual check, and the last result.
class UpdatePreferencesPage : public QWidget {
    Q_OBJECT

public:
    explicit UpdatePreferencesPage(UpdateChecker* checker, QWidget* parent = nullptr);

    void setAutomaticChecks(bool on);
    bool automaticChecks() const;
    QString resultText() const;
    QCheckBox* automaticChecksBox() const { return automaticChecks_; }
    QPushButton* checkNowButton() const { return checkNow_; }
    void refresh();

signals:
    void automaticChecksChanged(bool on);
    void checkRequested();

private:
    UpdateChecker* checker_ = nullptr;
    QCheckBox* automaticChecks_ = nullptr;
    QLabel* version_ = nullptr;
    QLabel* package_ = nullptr;
    QPushButton* checkNow_ = nullptr;
    QLabel* result_ = nullptr;
};

} // namespace pictura
