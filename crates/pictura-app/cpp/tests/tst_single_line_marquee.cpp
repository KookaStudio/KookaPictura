#include <QtTest/QtTest>
#include <QtCore/QTimer>
#include <QtGui/QFocusEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainterPath>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QMenu>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QToolBar>
#include <QtWidgets/QToolButton>

#include "selftest_paint_fixture.h"
#include "qt_test_support.h"
#include "options_bar.h"
#include "panels/numeric_field.h"
#include "refine_edge_dialog.h"
#include "toolbox.h"

using namespace pictura;
using paint_fixture::Fixture;

class SingleLineMarqueeTest : public QObject {
    Q_OBJECT
private slots:
    void initTestCase();
    void discovery();
    void row();
    void column();
    void onePixel();
    void history();
    void dragRow();
    void dragColumn();
    void dragCombine();
    void dragCombineLargeFeather();
    void dragEscape();
    void dragTargetLoss();
    void insidePressPreview();
    void unrelatedRelease();
    void modes();
    void feather();
    void featherFull();
    void featherWarning();
    void options();
    void refineEdge();
    void moveSelection();
    void moveOutAndBack();
    void smallMoveReturnsToClick();
    void documentDragCancelsBand();
    void sharedOptions();
    void contextMenu();
private:
    void click(Fixture& f, ToolId id, QPointF pos, Qt::KeyboardModifiers mods = Qt::NoModifier);
    void band(ToolId id);
    void dragBand(bool row);
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<PicturaMainWindow> window_;
};

void SingleLineMarqueeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

void SingleLineMarqueeTest::click(Fixture& f, ToolId id, QPointF pos, Qt::KeyboardModifiers mods)
{
    window_->setActiveTool(id);
    f.canvas->mousePressed(pos, Qt::LeftButton, int(mods));
    f.canvas->mouseReleased(pos);
}

void SingleLineMarqueeTest::discovery()
{
    auto* box = window_->findChild<Toolbox*>();
    QVERIFY(box);
    auto* slot = box->slotButton(2);
    QVERIFY(slot);
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QVERIFY(toolShortcut(id).isNull());
        QAction* found = nullptr;
        for (QAction* action : box->slotMenuActionsForTest(2)) {
            if (action->data().toInt() == int(id)) found = action;
        }
        QVERIFY(found);
        found->trigger();
        QCOMPARE(window_->activeTool(), id);
        for (bool shiftRequired : {false, true}) {
            box->setShiftKeyForToolSwitch(shiftRequired);
            window_->setActiveTool(id);
            QVERIFY(box->handleToolKey(QChar(u'M'), shiftRequired));
            QCOMPARE(window_->activeTool(), ToolId::Marquee);
            if (shiftRequired) {
                window_->setActiveTool(id);
                QVERIFY(box->handleToolKey(QChar(u'M'), false));
                QCOMPARE(window_->activeTool(), ToolId::Marquee);
            }
        }
    }
    box->setShiftKeyForToolSwitch(true);
    window_->setActiveTool(ToolId::Marquee);
    QVERIFY(box->handleToolKey(QChar(u'M'), true));
    QCOMPARE(window_->activeTool(), ToolId::EllipticalMarquee);
    QVERIFY(box->handleToolKey(QChar(u'M'), true));
    QCOMPARE(window_->activeTool(), ToolId::Marquee);
    QTest::mouseClick(slot, Qt::LeftButton, Qt::AltModifier);
    QCOMPARE(window_->activeTool(), ToolId::EllipticalMarquee);
    QTest::mouseClick(slot, Qt::LeftButton, Qt::AltModifier);
    QCOMPARE(window_->activeTool(), ToolId::SingleRowMarquee);
    QTest::mouseClick(slot, Qt::LeftButton, Qt::AltModifier);
    QCOMPARE(window_->activeTool(), ToolId::SingleColumnMarquee);
}

void SingleLineMarqueeTest::band(ToolId id)
{
    QImage seed(8, 6, QImage::Format_RGB32); seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("single_line_band"));
    QVERIFY(f.ok());
    f.tools->optionsFor<SelectionOptions>(id)->setFeather(0);
    f.tools->optionsFor<SelectionOptions>(id)->setCombineMode(SelectionMode::New);
    click(f, id, QPointF(3, 2));
    for (int y = 0; y < 6; ++y)
        for (int x = 0; x < 8; ++x)
            QCOMPARE(f.view->selection_coverage(x, y), (id == ToolId::SingleRowMarquee ? y == 2 : x == 3) ? 255 : 0);
}
void SingleLineMarqueeTest::row() { band(ToolId::SingleRowMarquee); }
void SingleLineMarqueeTest::column() { band(ToolId::SingleColumnMarquee); }

void SingleLineMarqueeTest::onePixel()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(1, 1, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_one")); QVERIFY(f.ok());
        f.tools->optionsFor<SelectionOptions>(id)->setFeather(0);
        f.tools->optionsFor<SelectionOptions>(id)->setCombineMode(SelectionMode::New);
        click(f, id, QPointF(0, 0));
        QCOMPARE(f.view->selection_coverage(0, 0), 255);
    }
}

void SingleLineMarqueeTest::history()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(8, 6, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_history")); QVERIFY(f.ok());
        QVERIFY(f.view->select_rect(1, 1, 4, 4, QStringLiteral("new"), 0));
        auto* opt = f.tools->optionsFor<SelectionOptions>(id); opt->setFeather(0); opt->setCombineMode(SelectionMode::New);
        click(f, id, QPointF(3, 2));
        QCOMPARE(f.view->selection_coverage(1, 1), 0);
        QVERIFY(f.view->undo()); QCOMPARE(f.view->selection_coverage(1, 1), 255);
        QVERIFY(f.view->redo()); QCOMPARE(f.view->selection_coverage(1, 1), 0);
    }
}

namespace {

QList<int> coverage(const Fixture& f, const QSize& size)
{
    QList<int> values;
    for (int y = 0; y < size.height(); ++y) {
        for (int x = 0; x < size.width(); ++x) {
            values.append(f.view->selection_coverage(x, y));
        }
    }
    return values;
}

QPainterPath previewPath(const ImageView& canvas)
{
    QPainterPath path;
    for (const QPolygonF& loop : canvas.selectionPreviewContours()) {
        path.addPolygon(loop);
        path.closeSubpath();
    }
    return path;
}

} // namespace

void SingleLineMarqueeTest::dragBand(bool row)
{
    const ToolId id = row ? ToolId::SingleRowMarquee : ToolId::SingleColumnMarquee;
    QImage seed(10, 8, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("single_line_live"));
    QVERIFY(f.ok());
    QVERIFY(f.view->select_rect(6, 5, 2, 2, QStringLiteral("new"), 0));
    const auto before = coverage(f, seed.size());
    const int history = f.view->history_index();
    auto* options = f.tools->optionsFor<SelectionOptions>(id);
    options->setCombineMode(SelectionMode::New);
    options->setFeather(0);
    window_->setActiveTool(id);

    f.canvas->mousePressed(QPointF(2, 1), Qt::LeftButton, int(Qt::NoModifier));
    const QRectF press = row ? QRectF(0, 1, 10, 1) : QRectF(2, 0, 1, 8);
    QCOMPARE(f.canvas->selectionBandGuide().boundingRect(), press);
    QCOMPARE(previewPath(*f.canvas).boundingRect(), press);

    f.canvas->mouseMoved(QPointF(4, 3));
    const QRectF moved = row ? QRectF(0, 3, 10, 1) : QRectF(4, 0, 1, 8);
    QCOMPARE(f.canvas->selectionBandGuide().boundingRect(), moved);
    QCOMPARE(previewPath(*f.canvas).boundingRect(), moved);
    f.canvas->mouseMoved(row ? QPointF(-20, 4) : QPointF(3, 20));
    const QRectF offAxis = row ? QRectF(0, 4, 10, 1) : QRectF(3, 0, 1, 8);
    QCOMPARE(f.canvas->selectionBandGuide().boundingRect(), offAxis);
    QCOMPARE(f.view->history_index(), history);
    QCOMPARE(coverage(f, seed.size()), before);

    f.canvas->mouseReleased(row ? QPointF(5, 100) : QPointF(-100, 5));
    QVERIFY(f.canvas->selectionBandGuide().isEmpty());
    QVERIFY(!f.canvas->hasSelectionPreviewForTest());
    QCOMPARE(f.view->history_index(), history + 1);
    for (int y = 0; y < seed.height(); ++y) {
        for (int x = 0; x < seed.width(); ++x) {
            const bool selected = row ? y == seed.height() - 1 : x == 0;
            QCOMPARE(f.view->selection_coverage(x, y), selected ? 255 : 0);
        }
    }
    QVERIFY(f.view->undo());
    QCOMPARE(coverage(f, seed.size()), before);
}

void SingleLineMarqueeTest::dragRow() { dragBand(true); }
void SingleLineMarqueeTest::dragColumn() { dragBand(false); }

void SingleLineMarqueeTest::dragCombine()
{
    for (bool row : {true, false}) {
        const ToolId id = row ? ToolId::SingleRowMarquee : ToolId::SingleColumnMarquee;
        for (auto mode : {SelectionMode::Add, SelectionMode::Subtract, SelectionMode::Intersect}) {
            QImage seed(10, 10, QImage::Format_RGB32);
            seed.fill(Qt::white);
            Fixture f(*window_, seed, QStringLiteral("single_line_live_combine"));
            QVERIFY(f.ok());
            QVERIFY(f.view->select_rect(2, 2, 6, 6, QStringLiteral("new"), 0));
            const auto before = coverage(f, seed.size());
            const int history = f.view->history_index();
            f.tools->optionsFor<SelectionOptions>(id)->setFeather(0);
            window_->setActiveTool(id);
            const auto mods = mode == SelectionMode::Add ? Qt::KeyboardModifiers(Qt::ShiftModifier)
                : mode == SelectionMode::Subtract ? Qt::KeyboardModifiers(Qt::AltModifier)
                                                  : (Qt::ShiftModifier | Qt::AltModifier);
            auto selected = [mode, row](int x, int y, int axis) {
                const bool old = x >= 2 && x < 8 && y >= 2 && y < 8;
                const bool band = (row ? y : x) == axis;
                if (mode == SelectionMode::Add) {
                    return old || band;
                }
                return mode == SelectionMode::Subtract ? old && !band : old && band;
            };

            f.canvas->mousePressed(QPointF(3, 3), Qt::LeftButton, int(mods));
            for (int axis : {3, 8, 6}) {
                if (axis != 3) {
                    f.canvas->mouseMoved(row ? QPointF(4, axis) : QPointF(axis, 4));
                }
                const QPainterPath path = previewPath(*f.canvas);
                for (int y = 0; y < seed.height(); ++y) {
                    for (int x = 0; x < seed.width(); ++x) {
                        QCOMPARE(path.contains(QPointF(x + 0.5, y + 0.5)), selected(x, y, axis));
                    }
                }
                QCOMPARE(coverage(f, seed.size()), before);
                QCOMPARE(f.view->history_index(), history);
            }
            f.canvas->mouseReleased(row ? QPointF(4, 6) : QPointF(6, 4));
            QCOMPARE(f.view->history_index(), history + 1);
            for (int y = 0; y < seed.height(); ++y) {
                for (int x = 0; x < seed.width(); ++x) {
                    QCOMPARE(f.view->selection_coverage(x, y), selected(x, y, 6) ? 255 : 0);
                }
            }
        }
    }
}

void SingleLineMarqueeTest::dragCombineLargeFeather()
{
    for (bool row : {true, false}) {
        const ToolId id = row ? ToolId::SingleRowMarquee : ToolId::SingleColumnMarquee;
        for (bool add : {true, false}) {
            QImage seed(512, 384, QImage::Format_RGB32); seed.fill(Qt::white);
            Fixture f(*window_, seed, QStringLiteral("single_line_large_feather")); QVERIFY(f.ok());
            QVERIFY(f.view->select_rect(12, 12, 488, 360, QStringLiteral("new"), 3));
            QVERIFY(f.view->select_rect(160, 100, 140, 120, QStringLiteral("subtract"), 0));
            auto* options = f.tools->optionsFor<SelectionOptions>(id);
            options->setFeather(1); options->setCombineMode(SelectionMode::New);
            window_->setActiveTool(id);
            const int history = f.view->history_index();
            const auto mods = add ? Qt::ShiftModifier : Qt::AltModifier;
            f.canvas->mousePressed(QPointF(30, 30), Qt::LeftButton, int(mods));
            const QPointF final = row ? QPointF(250, 160) : QPointF(230, 180);
            f.canvas->mouseMoved(row ? QPointF(200, 70) : QPointF(80, 160));
            f.canvas->mouseMoved(final);
            const QPainterPath preview = previewPath(*f.canvas);
            QCOMPARE(f.view->history_index(), history);
            f.canvas->mouseReleased(final);
            QCOMPARE(f.view->history_index(), history + 1);
            auto compare = [&](int x, int y) {
                QCOMPARE(preview.contains(QPointF(x + 0.5, y + 0.5)),
                         f.view->selection_coverage(x, y) >= 128);
            };
            for (int y = 0; y < seed.height(); y += 7) {
                for (int x = 0; x < seed.width(); x += 7) compare(x, y);
            }
            if (row) {
                for (int y = 154; y <= 166; ++y)
                    for (int x = 0; x < seed.width(); ++x) compare(x, y);
            } else {
                for (int x = 224; x <= 236; ++x)
                    for (int y = 0; y < seed.height(); ++y) compare(x, y);
            }
        }
    }
}

void SingleLineMarqueeTest::dragEscape()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(20, 20, QImage::Format_RGB32);
        seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_live_escape"));
        QVERIFY(f.ok());
        QVERIFY(f.view->select_rect(5, 5, 10, 10, QStringLiteral("new"), 2));
        const auto before = coverage(f, seed.size());
        const int history = f.view->history_index();
        auto* options = f.tools->optionsFor<SelectionOptions>(id);
        options->setCombineMode(SelectionMode::New);
        options->setFeather(0);
        window_->setActiveTool(id);
        f.canvas->mousePressed(QPointF(0, 0), Qt::LeftButton, int(Qt::NoModifier));
        f.canvas->mouseMoved(QPointF(8, 8));
        QVERIFY(!f.canvas->selectionBandGuide().isEmpty());
        QTest::keyClick(window_.get(), Qt::Key_Escape);
        QVERIFY(f.canvas->selectionBandGuide().isEmpty());
        QVERIFY(!f.canvas->hasSelectionPreviewForTest());
        f.canvas->mouseMoved(QPointF(10, 10));
        f.canvas->mouseReleased(QPointF(10, 10));
        QCOMPARE(coverage(f, seed.size()), before);
        QCOMPARE(f.view->history_index(), history);

        f.canvas->mousePressed(QPointF(9, 9), Qt::LeftButton, int(Qt::NoModifier));
        QVERIFY(f.canvas->hasSelectionPreviewForTest());
        QVERIFY(f.tools->handleEscape());
        f.canvas->mouseReleased(QPointF(9, 9));
        QCOMPARE(coverage(f, seed.size()), before);
        QCOMPARE(f.view->history_index(), history);
    }
}

void SingleLineMarqueeTest::dragTargetLoss()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        for (int reason = 0; reason < 3; ++reason) {
            QImage seed(16, 16, QImage::Format_RGB32);
            seed.fill(Qt::white);
            Fixture f(*window_, seed, QStringLiteral("single_line_live_target"));
            QVERIFY(f.ok());
            QVERIFY(f.view->select_rect(6, 6, 4, 4, QStringLiteral("new"), 0));
            const auto before = coverage(f, seed.size());
            const int history = f.view->history_index();
            auto* options = f.tools->optionsFor<SelectionOptions>(id);
            options->setCombineMode(SelectionMode::New);
            options->setFeather(0);
            window_->setActiveTool(id);
            f.canvas->mousePressed(QPointF(1, 1), Qt::LeftButton, int(Qt::NoModifier));
            f.canvas->mouseMoved(QPointF(4, 4));
            QVERIFY(!f.canvas->selectionBandGuide().isEmpty());
            if (reason == 0) {
                QFocusEvent event(QEvent::FocusOut);
                QApplication::sendEvent(f.canvas, &event);
            } else if (reason == 1) {
                window_->setActiveTool(ToolId::EllipticalMarquee);
            } else {
                Fixture other(*window_, seed, QStringLiteral("single_line_live_other"));
                QVERIFY(other.ok());
                QVERIFY(f.canvas->selectionBandGuide().isEmpty());
                const int otherHistory = other.view->history_index();
                other.canvas->mouseReleased(QPointF(7, 7));
                QCOMPARE(other.view->history_index(), otherHistory);
                QVERIFY(!other.view->has_selection());
            }
            QVERIFY(f.canvas->selectionBandGuide().isEmpty());
            f.canvas->mouseReleased(QPointF(7, 7));
            QCOMPARE(coverage(f, seed.size()), before);
            QCOMPARE(f.view->history_index(), history);
        }
    }
}

void SingleLineMarqueeTest::insidePressPreview()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(32, 32, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_inside_preview")); QVERIFY(f.ok());
        QVERIFY(f.view->select_rect(5, 5, 20, 20, QStringLiteral("new"), 0));
        auto* options = f.tools->optionsFor<SelectionOptions>(id);
        options->setCombineMode(SelectionMode::New); options->setFeather(8);
        window_->setActiveTool(id);
        const int history = f.view->history_index();
        f.canvas->mousePressed(QPointF(16, 16), Qt::LeftButton, int(Qt::NoModifier));
        QVERIFY(!f.canvas->selectionBandGuide().isEmpty());
        QVERIFY(f.canvas->selectionPreviewContours().isEmpty());
        options->setFeather(0);
        options->setCombineMode(SelectionMode::Add);
        QTimer::singleShot(0, [] {
            if (auto* message = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) message->accept();
        });
        f.canvas->mouseReleased(QPointF(16, 16));
        QCOMPARE(f.view->history_index(), history + 1);
        QVERIFY(f.view->selection_coverage(16, 16) > 0);
        QVERIFY(f.view->selection_coverage(16, 16) <= 127);
        QCOMPARE(f.view->selection_coverage(5, 5), 0);
    }
}

void SingleLineMarqueeTest::unrelatedRelease()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(32, 32, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_other_button")); QVERIFY(f.ok());
        auto* options = f.tools->optionsFor<SelectionOptions>(id);
        options->setCombineMode(SelectionMode::New); options->setFeather(0);
        window_->setActiveTool(id);
        const int history = f.view->history_index();
        f.canvas->mousePressed(QPointF(3, 3), Qt::LeftButton, int(Qt::NoModifier));
        const QPointF widget = f.canvas->imageToWidget(QPointF(5, 5));
        QMouseEvent release(QEvent::MouseButtonRelease, widget, Qt::RightButton,
                            Qt::LeftButton, Qt::NoModifier);
        QApplication::sendEvent(f.canvas, &release);
        QCOMPARE(f.view->history_index(), history);
        QVERIFY(!f.canvas->selectionBandGuide().isEmpty());
        f.canvas->mouseReleased(QPointF(6, 6));
        QCOMPARE(f.view->history_index(), history + 1);
        QCOMPARE(f.view->selection_coverage(6, 6), 255);
    }
}

void SingleLineMarqueeTest::modes()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        for (int mode = 0; mode < 4; ++mode) for (bool modifier : {false, true}) {
            QImage seed(8, 6, QImage::Format_RGB32); seed.fill(Qt::white);
            Fixture f(*window_, seed, QStringLiteral("single_line_modes")); QVERIFY(f.ok());
            QVERIFY(f.view->select_rect(1, 1, 4, 4, QStringLiteral("new"), 0));
            auto* opt = f.tools->optionsFor<SelectionOptions>(id); opt->setFeather(0);
            opt->setCombineMode(modifier ? SelectionMode::New : SelectionMode(mode));
            const Qt::KeyboardModifiers mods = !modifier || mode == 0 ? Qt::NoModifier : mode == 1 ? Qt::ShiftModifier : mode == 2 ? Qt::AltModifier : Qt::ShiftModifier | Qt::AltModifier;
            click(f, id, QPointF(3, 2), mods);
            for (int y = 0; y < 6; ++y) for (int x = 0; x < 8; ++x) {
                const bool old = x >= 1 && x < 5 && y >= 1 && y < 5;
                const bool line = id == ToolId::SingleRowMarquee ? y == 2 : x == 3;
                const bool expected = mode == 0 ? line : mode == 1 ? old || line : mode == 2 ? old && !line : old && line;
                QCOMPARE(f.view->selection_coverage(x, y), expected ? 255 : 0);
            }
        }
    }
}

void SingleLineMarqueeTest::feather()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(32, 32, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_feather")); QVERIFY(f.ok());
        auto* opt = f.tools->optionsFor<SelectionOptions>(id); opt->setFeather(2); opt->setCombineMode(SelectionMode::New);
        QTimer::singleShot(0, [] { if (auto* m = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) m->accept(); });
        click(f, id, QPointF(16, 16));
        const int c = f.view->selection_coverage(16, 16); QVERIFY(c > 0 && c < 255);
        QVERIFY(f.view->select_rect(8, 8, 16, 16, QStringLiteral("new"), 0));
        opt->setCombineMode(SelectionMode::Subtract);
        click(f, id, QPointF(16, 16));
        QCOMPARE(f.view->selection_coverage(16, 16), 255 - c);
        QCOMPARE(f.view->selection_coverage(9, 9), 255);
    }
}

void SingleLineMarqueeTest::featherFull()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(id == ToolId::SingleRowMarquee ? 8 : 1, id == ToolId::SingleRowMarquee ? 1 : 8, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_full")); QVERIFY(f.ok());
        auto* opt = f.tools->optionsFor<SelectionOptions>(id); opt->setFeather(2); opt->setCombineMode(SelectionMode::New);
        click(f, id, QPointF(0, 0));
        for (int y = 0; y < seed.height(); ++y) for (int x = 0; x < seed.width(); ++x) QCOMPARE(f.view->selection_coverage(x, y), 255);
    }
}

void SingleLineMarqueeTest::featherWarning()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(32, 32, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_warning")); QVERIFY(f.ok());
        QVERIFY(f.view->select_rect(2, 2, 4, 4, QStringLiteral("new"), 0));
        auto* opt = f.tools->optionsFor<SelectionOptions>(id); opt->setFeather(8); opt->setCombineMode(SelectionMode::New);
        bool warned = false;
        QTimer::singleShot(0, [&warned] { if (auto* m = qobject_cast<QMessageBox*>(QApplication::activeModalWidget())) { warned = m->text().contains(QStringLiteral("No pixels are more than 50% selected")); m->accept(); } });
        click(f, id, QPointF(16, 16)); QVERIFY(warned);
        const int c = f.view->selection_coverage(16, 16); QVERIFY(c > 0 && c <= 127);
        QVERIFY(f.view->undo()); QCOMPARE(f.view->selection_coverage(2, 2), 255);
        QVERIFY(f.view->redo()); QCOMPARE(f.view->selection_coverage(16, 16), c);
    }
}

void SingleLineMarqueeTest::options()
{
    auto* bar = window_->findChild<OptionsBar*>(QStringLiteral("optionsBar")); QVERIFY(bar);
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        window_->setActiveTool(id);
        QWidget* page = bar->pageForTest(id); QVERIFY(page);
        QVERIFY(page->findChild<QWidget*>(QStringLiteral("optionsFeather")));
        QVERIFY(page->findChild<QToolButton*>(QStringLiteral("optionsRefineEdge")));
        QVERIFY(!page->findChild<QCheckBox*>(QStringLiteral("optionsSelectionAntiAlias")));
        int modes = 0;
        for (auto* b : page->findChildren<QToolButton*>()) if (b->property("selectionMode").isValid()) ++modes;
        QCOMPARE(modes, 4);
        for (auto* c : page->findChildren<QComboBox*>()) QVERIFY(c->findText(QStringLiteral("Fixed Ratio")) < 0);
    }
}

void SingleLineMarqueeTest::refineEdge()
{
    QImage seed(32, 32, QImage::Format_RGB32); seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("single_line_refine")); QVERIFY(f.ok());
    auto* bar = window_->findChild<OptionsBar*>(QStringLiteral("optionsBar")); QVERIFY(bar);
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        auto* opt = f.tools->optionsFor<SelectionOptions>(id); opt->setFeather(0); opt->setCombineMode(SelectionMode::New);
        click(f, id, QPointF(16, 16));
        QWidget* page = bar->pageForTest(id); QVERIFY(page);
        auto* button = page->findChild<QToolButton*>(QStringLiteral("optionsRefineEdge"));
        QVERIFY(button); QTRY_VERIFY(button->isEnabled());
        bool opened = false;
        QTimer::singleShot(0, [&opened] {
            for (QWidget* widget : QApplication::topLevelWidgets()) {
                if (auto* dialog = qobject_cast<RefineEdgeDialog*>(widget)) { opened = true; dialog->reject(); }
            }
        });
        button->click(); QVERIFY(opened);
    }
}

void SingleLineMarqueeTest::moveSelection()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(32, 32, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_move")); QVERIFY(f.ok());
        QVERIFY(f.view->select_rect(5, 5, 5, 5, QStringLiteral("new"), 0));
        window_->setActiveTool(id);
        f.drag({QPointF(6, 6), QPointF(12, 12)});
        QCOMPARE(f.view->selection_coverage(6, 6), 0); QCOMPARE(f.view->selection_coverage(12, 12), 255);
    }
}

void SingleLineMarqueeTest::moveOutAndBack()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(32, 32, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_return")); QVERIFY(f.ok());
        QVERIFY(f.view->select_rect(5, 5, 5, 5, QStringLiteral("new"), 0));
        auto* opt = f.tools->optionsFor<SelectionOptions>(id);
        opt->setFeather(0); opt->setCombineMode(SelectionMode::New);
        window_->setActiveTool(id);
        const int history = f.view->history_index();
        f.drag({QPointF(6, 6), QPointF(25, 25), QPointF(6, 6)});
        QCOMPARE(f.view->selection_coverage(6, 6), 255);
        QCOMPARE(f.view->selection_coverage(0, 6), 0);
        QCOMPARE(f.view->selection_coverage(6, 0), 0);
        QCOMPARE(f.view->history_index(), history);
    }
}

void SingleLineMarqueeTest::smallMoveReturnsToClick()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(32, 32, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_small_return")); QVERIFY(f.ok());
        QVERIFY(f.view->select_rect(5, 5, 5, 5, QStringLiteral("new"), 0));
        auto* options = f.tools->optionsFor<SelectionOptions>(id);
        options->setFeather(0); options->setCombineMode(SelectionMode::New);
        window_->setActiveTool(id);
        f.canvas->setZoom(1.0, QPointF());
        const int history = f.view->history_index();
        f.canvas->mousePressed(QPointF(6, 6), Qt::LeftButton, int(Qt::NoModifier));
        f.canvas->mouseMoved(QPointF(7, 6));
        options->setCombineMode(SelectionMode::Add);
        f.canvas->mouseMoved(QPointF(6, 6));
        f.canvas->mouseReleased(QPointF(6, 6));
        QCOMPARE(f.view->history_index(), history + 1);
        QCOMPARE(f.view->selection_coverage(0, 6), id == ToolId::SingleRowMarquee ? 255 : 0);
        QCOMPARE(f.view->selection_coverage(6, 0), id == ToolId::SingleColumnMarquee ? 255 : 0);
        QCOMPARE(f.view->selection_coverage(5, 5), 0);
    }
}

void SingleLineMarqueeTest::documentDragCancelsBand()
{
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        QImage seed(32, 32, QImage::Format_RGB32); seed.fill(Qt::white);
        Fixture f(*window_, seed, QStringLiteral("single_line_document_drag")); QVERIFY(f.ok());
        QVERIFY(f.view->select_rect(5, 5, 5, 5, QStringLiteral("new"), 0));
        auto* options = f.tools->optionsFor<SelectionOptions>(id);
        options->setFeather(0); options->setCombineMode(SelectionMode::New);
        window_->setActiveTool(id);
        const int history = f.view->history_index();
        f.canvas->mousePressed(QPointF(6, 6), Qt::LeftButton, int(Qt::NoModifier));
        QVERIFY(f.tools->prepareSelectionDocumentDragForTest());
        QVERIFY(f.canvas->selectionBandGuide().isEmpty());
        f.canvas->mouseReleased(QPointF(6, 6));
        QCOMPARE(f.view->history_index(), history);
        QCOMPARE(f.view->selection_coverage(5, 5), 255);
        QCOMPARE(f.view->selection_coverage(0, 6), 0);
    }
}

void SingleLineMarqueeTest::sharedOptions()
{
    auto* bar = window_->findChild<OptionsBar*>(QStringLiteral("optionsBar")); QVERIFY(bar);
    window_->setActiveTool(ToolId::SingleRowMarquee);
    QWidget* row = bar->pageForTest(ToolId::SingleRowMarquee); QVERIFY(row);
    auto* feather = row->findChild<NumericField*>(QStringLiteral("optionsFeather")); QVERIFY(feather);
    auto* edit = feather->findChild<QLineEdit*>(); QVERIFY(edit);
    edit->setFocus(); QTest::keyClick(edit, Qt::Key_A, Qt::ControlModifier);
    QTest::keyClicks(edit, QStringLiteral("3")); QTest::keyClick(edit, Qt::Key_Return);
    QToolButton* subtract = nullptr;
    for (auto* button : row->findChildren<QToolButton*>()) {
        if (button->property("selectionMode").isValid()
            && button->property("selectionMode").toInt() == int(SelectionMode::Subtract)) subtract = button;
    }
    QVERIFY(subtract); subtract->click();
    for (ToolId id : {ToolId::SingleColumnMarquee, ToolId::Marquee}) {
        window_->setActiveTool(id);
        QWidget* page = bar->pageForTest(id); QVERIFY(page);
        auto* field = page->findChild<NumericField*>(QStringLiteral("optionsFeather")); QVERIFY(field);
        QCOMPARE(field->value(), 3.0);
        bool checked = false;
        for (auto* button : page->findChildren<QToolButton*>()) {
            if (button->property("selectionMode").isValid()
                && button->property("selectionMode").toInt() == int(SelectionMode::Subtract)) checked = button->isChecked();
        }
        QVERIFY(checked);
    }
}

void SingleLineMarqueeTest::contextMenu()
{
    QImage seed(16, 16, QImage::Format_RGB32); seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("single_line_context")); QVERIFY(f.ok());
    QVERIFY(f.view->select_rect(2, 2, 5, 5, QStringLiteral("new"), 0));
    for (ToolId id : {ToolId::SingleRowMarquee, ToolId::SingleColumnMarquee}) {
        window_->setActiveTool(id);
        std::unique_ptr<QMenu> menu(f.tools->contextMenuForTest(f.canvas, QPointF(3, 3)));
        QVERIFY(menu); QVERIFY(!menu->actions().isEmpty());
    }
}

PICTURA_QT_TEST_MAIN(SingleLineMarqueeTest)
#include "tst_single_line_marquee.moc"
