#pragma once

#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QCheckBox;
class QLabel;
class QProgressBar;
class QPushButton;

namespace pictura {

// The update notice is a compact, non-modal popup dialog. It never takes focus
// from the document, but it is a real window rather than a side overlay.
class UpdateBanner : public QDialog {
    Q_OBJECT

public:
    explicit UpdateBanner(QWidget* parent = nullptr);

    void showUpdate(const QString& currentVersion, const QString& availableVersion,
                    bool canInstall, const QString& command);
    void showProgress(qint64 received, qint64 total);
    void showError(const QString& message);
    void setAutomaticChecks(bool on);

    QString titleText() const;
    QString commandText() const;
    QString errorText() const;
    QPushButton* updateButton() const { return update_; }
    QPushButton* laterButton() const { return later_; }
    QPushButton* notesButton() const { return notes_; }
    QPushButton* releasePageButton() const { return releasePage_; }
    QPushButton* cancelButton() const { return cancel_; }
    QCheckBox* automaticChecksBox() const { return automaticChecks_; }
    QProgressBar* progressBar() const { return progress_; }

signals:
    void updateRequested();
    void releaseNotesRequested();
    void releasePageRequested();
    void laterRequested();
    void cancelRequested();
    void automaticChecksChanged(bool on);

private:
    void setBusy(bool busy);
    void resetToAvailable();
    void centerOnParent();

    QLabel* title_ = nullptr;
    QLabel* command_ = nullptr;
    QLabel* error_ = nullptr;
    QProgressBar* progress_ = nullptr;
    QCheckBox* automaticChecks_ = nullptr;
    QPushButton* notes_ = nullptr;
    QPushButton* later_ = nullptr;
    QPushButton* update_ = nullptr;
    QPushButton* releasePage_ = nullptr;
    QPushButton* cancel_ = nullptr;
    QString currentVersion_;
    QString availableVersion_;
    bool canInstall_ = false;
};

} // namespace pictura
