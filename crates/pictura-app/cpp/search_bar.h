#pragma once

// The Help > Search (Shift+Space) overlay: a query field over a result list
// of breadcrumbs. Not a CS6 feature (#788); post-CS6 by owner decision.

#include "search_index.h"

#include <QtCore/QList>
#include <QtCore/QPointer>
#include <QtCore/QRect>
#include <QtWidgets/QFrame>

class QLabel;
class QLineEdit;
class QListWidget;

namespace pictura {

// An in-window overlay, not a Qt::Popup: a popup grabs the pointer. The query
// field keeps the focus, so the field's keys drive the list.
class SearchBar : public QFrame {
    Q_OBJECT

public:
    // Whether a result row's entry can run (greyed rows cannot).
    static constexpr int kRunnableRole = Qt::UserRole + 1;
    static constexpr int kShortcutRole = Qt::UserRole + 2;

    explicit SearchBar(QWidget* parent);

    // Show over `host` (parent coordinates) with an empty query, focused.
    void open(const QList<SearchEntry>& entries, const QRect& host);
    // Close and hand the focus back to where it was; a no-op when closed.
    void dismiss();

    QLineEdit* input() const { return input_; }
    QListWidget* results() const { return results_; }
    // The entry behind result row `row`, or null.
    const SearchEntry* entryAt(int row) const;

Q_SIGNALS:
    void runRequested(const pictura::SearchEntry& entry);
    void revealRequested(const pictura::SearchEntry& entry);

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    bool handleKey(QKeyEvent* event);
    void updateResults();
    void moveSelection(int delta);
    void activate(int row, bool reveal);
    void relayout();

    QLineEdit* input_ = nullptr;
    QListWidget* results_ = nullptr;
    QLabel* empty_ = nullptr;
    QList<SearchEntry> entries_;
    QList<int> rows_;
    QRect host_;
    QPointer<QWidget> previousFocus_;
    bool closing_ = false;
};

// Outline `target` with a short accent pulse (Alt+Enter reveal).
void flashWidget(QWidget* target);

} // namespace pictura
