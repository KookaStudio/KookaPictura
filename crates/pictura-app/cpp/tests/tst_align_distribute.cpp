// Layer > Align / Align Layers To Selection / Distribute and the Move tool's
// buttons (#64). Each layer is a document-sized Layer via Copy of one square,
// so its rect offset is how far the square has moved.

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtGui/QAction>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

class AlignDistributeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void menuCommands();
    void alignToSelection();
    void moveToolButtons();

private:
    // Three 6x6 squares at (2, 4), (8, 12), (26, 24); returns their paths in
    // that order.
    QStringList setupSquares();
    QAction* leaf(const QString& submenu, const QString& edge) const;
    // The layer's rect offset as (left, top).
    QPoint offset(const QString& path) const;
    void select(const QStringList& paths);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void AlignDistributeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void AlignDistributeTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
    panel_ = nullptr;
}

QStringList AlignDistributeTest::setupSquares()
{
    if (!window_->newDocument(QStringLiteral("Align"), 40, 40, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return {};
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_) {
        return {};
    }
    // Each copy lands directly above the Background, pushing the earlier ones up.
    for (const QPoint at : {QPoint(2, 4), QPoint(8, 12), QPoint(26, 24)}) {
        view_->select_rect(at.x(), at.y(), 6, 6, QStringLiteral("new"), 0.0);
        if (view_->layer_via_copy(QStringLiteral("0")).isEmpty()) {
            return {};
        }
    }
    view_->deselect();
    QCoreApplication::processEvents();
    return {QStringLiteral("3"), QStringLiteral("2"), QStringLiteral("1")};
}

QAction* AlignDistributeTest::leaf(const QString& submenu, const QString& edge) const
{
    return window_->registry()->action(
        pictura::commandIdForPath({QStringLiteral("Layer"), submenu, edge}));
}

QPoint AlignDistributeTest::offset(const QString& path) const
{
    const QStringList parts = pictura::transform_api::layerRect(*view_, path).toString().split(QLatin1Char(' '));
    return parts.size() == 4 ? QPoint(parts[0].toInt(), parts[1].toInt()) : QPoint(-999, -999);
}

void AlignDistributeTest::select(const QStringList& paths)
{
    panel_->selectPaths(paths, paths.first());
    window_->registry()->refresh();
}

void AlignDistributeTest::menuCommands()
{
    const QStringList squares = setupSquares();
    QCOMPARE(squares.size(), 3);
    QAction* alignTop = leaf(QStringLiteral("Align"), QStringLiteral("Top"));
    QAction* toSelectionTop =
        leaf(QStringLiteral("Align Layers To Selection"), QStringLiteral("Top"));
    QAction* distributeLeft = leaf(QStringLiteral("Distribute"), QStringLiteral("Left"));
    QVERIFY(alignTop && toSelectionTop && distributeLeft);

    // One layer has nothing to line up with; two can align but not distribute.
    select({squares[0]});
    QVERIFY(!alignTop->isEnabled());
    select({squares[0], squares[1]});
    QVERIFY(alignTop->isEnabled());
    QVERIFY(!distributeLeft->isEnabled());
    QVERIFY(!toSelectionTop->isEnabled());

    select(squares);
    QVERIFY(distributeLeft->isEnabled());
    const int before = view_->history_count();
    alignTop->trigger();
    QCOMPARE(view_->history_count(), before + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Align Top Edges"));
    // Every top meets the highest square's, at y = 4.
    QCOMPARE(offset(squares[0]), QPoint(0, 0));
    QCOMPARE(offset(squares[1]), QPoint(0, -8));
    QCOMPARE(offset(squares[2]), QPoint(0, -20));
    QVERIFY(view_->undo());
    QCOMPARE(offset(squares[2]), QPoint(0, 0));

    // Left edges at 2, 8, 26: the middle one moves to 14, the ends stay.
    distributeLeft->trigger();
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Distribute Left Edges"));
    QCOMPARE(offset(squares[0]), QPoint(0, 0));
    QCOMPARE(offset(squares[1]), QPoint(6, 0));
    QCOMPARE(offset(squares[2]), QPoint(0, 0));
}

void AlignDistributeTest::alignToSelection()
{
    const QStringList squares = setupSquares();
    QCOMPARE(squares.size(), 3);
    QAction* alignBottom = leaf(QStringLiteral("Align"), QStringLiteral("Bottom"));
    QAction* toSelectionBottom =
        leaf(QStringLiteral("Align Layers To Selection"), QStringLiteral("Bottom"));
    QVERIFY(alignBottom && toSelectionBottom);

    // With a selection, one layer is enough: its bottom (10) meets the
    // selection's (40).
    view_->select_rect(0, 30, 40, 10, QStringLiteral("new"), 0.0);
    select({squares[0]});
    QVERIFY(!alignBottom->isEnabled());
    QVERIFY(toSelectionBottom->isEnabled());
    toSelectionBottom->trigger();
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Align Bottom Edges"));
    QCOMPARE(offset(squares[0]), QPoint(0, 30));
}

void AlignDistributeTest::moveToolButtons()
{
    const QStringList squares = setupSquares();
    QCOMPARE(squares.size(), 3);
    window_->setActiveTool(pictura::ToolId::Move);
    auto* alignLeft = window_->findChild<QToolButton*>(QStringLiteral("optionsAlignLeft"));
    auto* distributeTop = window_->findChild<QToolButton*>(QStringLiteral("optionsDistributeTop"));
    QVERIFY(alignLeft && distributeTop);
    QCOMPARE(alignLeft->icon().isNull(), false);

    select({squares[0]});
    QVERIFY(!alignLeft->isEnabled());
    select({squares[0], squares[1]});
    QVERIFY(alignLeft->isEnabled());
    QVERIFY(!distributeTop->isEnabled());
    select(squares);
    QVERIFY(distributeTop->isEnabled());

    alignLeft->click();
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Align Left Edges"));
    QCOMPARE(offset(squares[0]), QPoint(0, 0));
    QCOMPARE(offset(squares[1]), QPoint(-6, 0));
    QCOMPARE(offset(squares[2]), QPoint(-24, 0));

    // Tops at 4, 12, 24: the middle one moves to 14.
    distributeTop->click();
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Distribute Top Edges"));
    QCOMPARE(offset(squares[1]), QPoint(-6, 2));
}

QTEST_MAIN(AlignDistributeTest)
#include "tst_align_distribute.moc"
