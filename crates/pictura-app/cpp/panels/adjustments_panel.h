#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QVector>
#include <QtWidgets/QWidget>

class QToolButton;

namespace pictura {

class PictureView;

// CS6 Adjustments panel content: an always-visible launch grid of fill and
// adjustment icons. Clicking an entry emits a request; the frame creates the
// layer and selects it so the Properties panel opens.
class AdjustmentsPanel : public QWidget {
    Q_OBJECT

public:
    struct Entry {
        const char* id;
        const char* label;
        const char* iconId;
        bool fill;
    };

    static const Entry kEntries[19];

    explicit AdjustmentsPanel(QWidget* parent = nullptr);

    // A null view disables the grid (no document open).
    void setView(PictureView* view);

    int buttonCountForTest() const;
    QToolButton* buttonForTest(int index) const;
    QStringList buttonIdsForTest() const;
    QStringList buttonToolTipsForTest() const;
    QToolButton* buttonByIdForTest(const QString& id) const;

signals:
    void adjustmentRequested(const QString& kind);
    void fillRequested(const QString& type);

private:
    QToolButton* makeButton(const Entry& entry);

    QVector<QToolButton*> buttons_;
};

} // namespace pictura
