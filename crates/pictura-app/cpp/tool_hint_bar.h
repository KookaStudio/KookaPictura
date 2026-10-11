#pragma once

#include "tools.h"

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtWidgets/QWidget>

namespace pictura {

// Bottom-status-bar hint strip for the active tool. Each entry renders as a
// bordered keycap followed by its description; with no entries it falls back to
// a plain text hint. One keycap highlights while its key is held.
class ToolHintBar : public QWidget {
    Q_OBJECT

public:
    explicit ToolHintBar(QWidget* parent = nullptr);

    void setHints(const QList<ToolHint>& hints, const QString& fallbackText);
    // Highlight the keycap whose label matches `key` (case-insensitive); an
    // empty key clears the highlight.
    void setPressedKey(const QString& key);

    // Test hooks.
    int hintCountForTest() const { return hints_.size(); }
    QString keyForTest(int index) const;
    QString textForTest(int index) const;
    bool highlightedForTest(int index) const { return index == pressed_; }
    // The Move tool's `Arrows` key renders as directional chevrons; the count
    // and the directions both come from the painter's single source of truth.
    int chevronCountForTest(int index) const;
    // The chevron directions the painter draws for `Arrows`, in draw order
    // (up, left, down, right).
    QString chevronDirectionsForTest() const;
    // The Lucide asset and whether it is rotated 180° for each direction above.
    QString chevronAssetForTest(int index) const;
    bool chevronFlippedForTest(int index) const;
    QString fallbackForTest() const { return fallbackText_; }

    QSize sizeHint() const override;

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    QList<ToolHint> hints_;
    QString fallbackText_;
    int pressed_ = -1;
};

} // namespace pictura
