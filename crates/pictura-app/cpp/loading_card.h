#pragma once

#include <QtCore/QString>
#include <QtWidgets/QFrame>

class QLabel;
class QProgressBar;

namespace pictura {

// The loading tab's empty-workspace card: file name, phase, phase-weighted
// progress, percentage, and Cancel.
class LoadingCard : public QFrame {
    Q_OBJECT

public:
    explicit LoadingCard(const QString& fileName, QWidget* parent = nullptr);

    void setProgress(int percent, const QString& phase);

    int progressForTest() const;
    QString phaseForTest() const;
    QString percentForTest() const;

Q_SIGNALS:
    void cancelRequested();

private:
    QLabel* phase_ = nullptr;
    QLabel* percent_ = nullptr;
    QProgressBar* bar_ = nullptr;
};

} // namespace pictura
