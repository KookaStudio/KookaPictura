#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtCore/QScopeGuard>
#include <QtGui/QPainter>
#include <QtWidgets/QAbstractButton>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QSpinBox>

#include "api/history_api.h"
#include "api/paint_bridge.h"
#include "api/layers_bridge.h"
#include "api/transform_bridge.h"

#include <limits>
#include "commands.h"
#include "frame.h"
#include "qt_test_support.h"

namespace {
constexpr auto kCommand = "edit.puppet.warp";

QDialog* dialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (widget->objectName() == QStringLiteral("puppetWarpDialog") && widget->isVisible())
            return qobject_cast<QDialog*>(widget);
    }
    return nullptr;
}

QPoint point(QWidget* canvas, double x, double y)
{
    const double scale = qMin((canvas->width() - 96.0) / 160.0,
                              (canvas->height() - 96.0) / 120.0);
    return QPoint(qRound(canvas->width() / 2.0 + (x - 80.0) * scale),
                  qRound(canvas->height() / 2.0 + (y - 60.0) * scale));
}

void drag(QWidget* canvas, QPoint from, QPoint to, Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    QTest::mousePress(canvas, Qt::LeftButton, mods, from);
    QTest::mouseMove(canvas, to, 20);
    QTest::mouseRelease(canvas, Qt::LeftButton, mods, to);
}

void deform(QWidget* canvas)
{
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, point(canvas, 40, 60));
    QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, point(canvas, 110, 60));
    drag(canvas, point(canvas, 110, 60), point(canvas, 125, 75));
}
} // namespace

class PuppetWarpTest : public QObject {
    Q_OBJECT
private slots:
    void initTestCase();
    void init();
    void cleanup();
    void targetEligibility();
    void meshControls();
    void applyCancelUndo();
    void refusedOrIdentityAddsNoHistory();
    void staleTargetRefused();
    void maskApplyUndo();
    void linkedMaskApplyUndo();
    void autoRotationGesture();
    void foldPreviewCommit();
    void samplingPreviewCommit();
private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void PuppetWarpTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
}

void PuppetWarpTest::init()
{
    QVERIFY(window_->newDocument(QStringLiteral("Puppet"), 160, 120, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    auto* view = window_->activeView();
    QVERIFY(view);
    const int layer = view->add_layer(0);
    QVERIFY(layer > 0);
    window_->selectLayerPath(QString::number(layer));
    for (int x = 0; x < 160; x += 20) {
        QVERIFY(view->select_rect(x, 0, 10, 120, QStringLiteral("new"), 0.0));
        QVERIFY(pictura::edit_fill(*view, qRgba(30, 80, 220, 255), -1,
                                   QStringLiteral("normal"), 100, false));
    }
    view->deselect();
    QVERIFY(pictura::history_api::purgeUndo(*view));
}

void PuppetWarpTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0)
        window_->closeDocument(window_->activeDocumentIndex(), false);
}

void PuppetWarpTest::targetEligibility()
{
    auto* action = window_->registry()->action(QString::fromLatin1(kCommand));
    QVERIFY(action);
    QVERIFY(action->isEnabled());
    auto* view = window_->activeView();
    QVERIFY(view->set_layer_lock(1, QStringLiteral("all"), true));
    QCoreApplication::processEvents();
    QVERIFY(!action->isEnabled());
    QVERIFY(view->set_layer_lock(1, QStringLiteral("all"), false));
    view->set_layer_visible(1, false);
    QCoreApplication::processEvents();
    QVERIFY(!action->isEnabled());
    view->set_layer_visible(1, true);
    QCoreApplication::processEvents();
    QVERIFY(view->select_rect(10, 10, 30, 30, QStringLiteral("new"), 0.0));
    QCoreApplication::processEvents();
    QVERIFY(!action->isEnabled());
    view->deselect();
    QCoreApplication::processEvents();
    QVERIFY(action->isEnabled());
}

void PuppetWarpTest::meshControls()
{
    auto* view = window_->activeView();
    const QImage before = view->image();
    const int history = view->history_count();
    bool opened = false;
    QTimer::singleShot(0, [&] {
        auto* d = dialog();
        QVERIFY(d);
        const auto close = qScopeGuard([d] { if (d->isVisible()) d->reject(); });
        opened = true;
        auto* canvas = d->findChild<QWidget*>(QStringLiteral("puppetWarpCanvas"));
        auto* mode = d->findChild<QComboBox*>(QStringLiteral("puppetWarpMode"));
        auto* density = d->findChild<QComboBox*>(QStringLiteral("puppetWarpDensity"));
        auto* mesh = d->findChild<QCheckBox*>(QStringLiteral("puppetWarpShowMesh"));
        auto* forward = d->findChild<QAbstractButton*>(QStringLiteral("puppetWarpForward"));
        auto* backward = d->findChild<QAbstractButton*>(QStringLiteral("puppetWarpBackward"));
        QVERIFY(canvas && mode && density && mesh && forward && backward);
        for (const char* name : {"puppetWarpExpansion", "puppetWarpRotateMode",
                                 "puppetWarpAngle", "puppetWarpStatus"}) {
            QVERIFY(d->findChild<QWidget*>(QString::fromLatin1(name)));
        }
        QCOMPARE(mode->currentIndex(), 1);
        QCOMPARE(density->currentIndex(), 1);
        mode->setCurrentIndex(0);
        density->setCurrentIndex(2);
        QTest::mouseClick(canvas, Qt::LeftButton, Qt::NoModifier, point(canvas, 40, 60));
        QTest::mouseClick(canvas, Qt::LeftButton, Qt::ShiftModifier, point(canvas, 110, 60));
        const QImage pins = canvas->grab().toImage();
        QTest::keyPress(canvas, Qt::Key_H);
        const QImage hidden = canvas->grab().toImage();
        QVERIFY(hidden != pins);
        QTest::keyRelease(canvas, Qt::Key_H);
        QCOMPARE(canvas->grab().toImage(), pins);
        mode->setFocus();
        QTest::keyPress(mode, Qt::Key_H);
        QVERIFY(canvas->grab().toImage() != pins);
        QTest::keyRelease(mode, Qt::Key_H);
        QCOMPARE(canvas->grab().toImage(), pins);
        canvas->setFocus();
        mesh->setChecked(!mesh->isChecked());
        QVERIFY(canvas->grab().toImage() != pins);
        const QImage beforeMove = canvas->grab().toImage();
        drag(canvas, point(canvas, 110, 60), point(canvas, 120, 70), Qt::ShiftModifier);
        QVERIFY(canvas->grab().toImage() != beforeMove);
        QTest::keyClick(canvas, Qt::Key_Z, Qt::ControlModifier);
        QCOMPARE(canvas->grab().toImage(), beforeMove);
        forward->click();
        backward->click();
        const QPoint ring = point(canvas, 110, 60) + QPoint(20, 0);
        drag(canvas, ring, ring + QPoint(-5, 15), Qt::AltModifier);
        QTest::keyClick(canvas, Qt::Key_A, Qt::ControlModifier);
        QTest::keyClick(canvas, Qt::Key_D, Qt::ControlModifier);
        QTest::mouseClick(canvas, Qt::LeftButton, Qt::AltModifier, point(canvas, 40, 60));
        QCOMPARE(view->image(), before);
        QCOMPARE(view->history_count(), history);
        d->reject();
    });
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(kCommand)));
    QVERIFY(opened);
    QCOMPARE(view->image(), before);
    QCOMPARE(view->history_count(), history);
}

void PuppetWarpTest::applyCancelUndo()
{
    auto* view = window_->activeView();
    const QImage before = view->image();
    const int history = view->history_count();
    bool opened = false;
    QTimer::singleShot(0, [&] {
        auto* d = dialog();
        QVERIFY(d);
        const auto close = qScopeGuard([d] { if (d->isVisible()) d->reject(); });
        opened = true;
        auto* canvas = d->findChild<QWidget*>(QStringLiteral("puppetWarpCanvas"));
        QVERIFY(canvas);
        deform(canvas);
        QCOMPARE(view->image(), before);
        QCOMPARE(view->history_count(), history);
        d->reject();
    });
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(kCommand)));
    QVERIFY(opened);
    QCOMPARE(view->image(), before);
    QCOMPARE(view->history_count(), history);
    opened = false;
    QTimer::singleShot(0, [&] {
        auto* d = dialog();
        QVERIFY(d);
        const auto close = qScopeGuard([d] { if (d->isVisible()) d->reject(); });
        opened = true;
        auto* canvas = d->findChild<QWidget*>(QStringLiteral("puppetWarpCanvas"));
        QVERIFY(canvas);
        deform(canvas);
        QTest::keyClick(d, Qt::Key_Return);
    });
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(kCommand)));
    QVERIFY(opened);
    QCOMPARE(view->history_count(), history + 1);
    QVERIFY(view->image() != before);
    QVERIFY(view->undo());
    QCOMPARE(view->image(), before);
}

void PuppetWarpTest::refusedOrIdentityAddsNoHistory()
{
    auto* view = window_->activeView();
    const QImage before = view->image();
    const int history = view->history_count();
    bool opened = false;
    QTimer::singleShot(0, [&] {
        auto* d = dialog();
        QVERIFY(d);
        const auto close = qScopeGuard([d] { if (d->isVisible()) d->reject(); });
        opened = true;
        QTest::keyClick(d, Qt::Key_Return);
    });
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(kCommand)));
    QVERIFY(opened);
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), before);
    auto session = pictura::puppet_warp_begin(*view);
    const double nan = std::numeric_limits<double>::quiet_NaN();
    const double pins[] = {40, 60, 40, 60, nan, 0, 110, 60, 100000, 100000, nan, 1};
    QCOMPARE(pictura::puppet_warp_configure(*session, ::rust::Slice<const double>(pins, 12),
                                            1, 1, 0), 0);
    QVERIFY(!pictura::puppet_warp_commit(*view, *session));
    QCOMPARE(view->history_count(), history);
    QCOMPARE(view->image(), before);
}


void PuppetWarpTest::staleTargetRefused()
{
    auto* view = window_->activeView();
    auto session = pictura::puppet_warp_begin(*view);
    const double nan = std::numeric_limits<double>::quiet_NaN();
    const double pins[] = {40, 60, 40, 60, nan, 0, 110, 60, 125, 75, nan, 1};
    QCOMPARE(pictura::puppet_warp_configure(*session, ::rust::Slice<const double>(pins, 12),
                                            1, 1, 0), 2);
    QVERIFY(view->select_rect(5, 5, 10, 10, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::edit_fill(*view, qRgba(255, 0, 0, 255), -1,
                               QStringLiteral("normal"), 100, false));
    view->deselect();
    const QImage changed = view->image();
    const int history = view->history_count();
    QVERIFY(!pictura::puppet_warp_commit(*view, *session));
    QCOMPARE(view->image(), changed);
    QCOMPARE(view->history_count(), history);
}


void PuppetWarpTest::maskApplyUndo()
{
    auto* view = window_->activeView();
    const QString layer = QStringLiteral("1");
    const QImage colors = view->image();
    QVERIFY(view->select_rect(20, 20, 70, 70, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::layer_mask_add_path(*view, layer, QStringLiteral("reveal-selection")));
    view->deselect();
    QVERIFY(pictura::mask_edit_target_set(*view, layer));
    const QImage before = view->image();
    const int history = view->history_count();
    auto session = pictura::puppet_warp_begin(*view);
    const double nan = std::numeric_limits<double>::quiet_NaN();
    const double pins[] = {30, 50, 30, 50, nan, 0, 80, 50, 100, 65, nan, 1};
    QCOMPARE(pictura::puppet_warp_configure(*session, ::rust::Slice<const double>(pins, 12),
                                            1, 1, 0), 2);
    QVERIFY(pictura::puppet_warp_commit(*view, *session));
    QCOMPARE(view->history_count(), history + 1);
    QVERIFY(view->image() != before);
    QVERIFY(pictura::layer_mask_set_enabled_path(*view, layer, false));
    QCOMPARE(view->image(), colors);
    if (view->history_count() > history + 1) {
        QVERIFY(view->undo());
    } else {
        QVERIFY(pictura::layer_mask_set_enabled_path(*view, layer, true));
    }
    QVERIFY(view->undo());
    QCOMPARE(view->image(), before);
}


void PuppetWarpTest::linkedMaskApplyUndo()
{
    auto* view = window_->activeView();
    QVERIFY(view->select_rect(20, 20, 70, 70, QStringLiteral("new"), 0.0));
    QVERIFY(pictura::layer_mask_add_path(*view, QStringLiteral("1"), QStringLiteral("reveal-selection")));
    view->deselect();
    QVERIFY(pictura::mask_edit_target_set(*view, QString()));
    window_->selectLayerPath(QStringLiteral("1"));
    const QImage before = view->image();
    const int history = view->history_count();
    auto session = pictura::puppet_warp_begin(*view);
    QCOMPARE(pictura::puppet_warp_size(*session).size(), size_t(2));
    const double unused = 0;
    QCOMPARE(pictura::puppet_warp_configure(*session, ::rust::Slice<const double>(&unused, 0), 1, 1, 0), 1);
    const QImage initialPreview = pictura::puppet_warp_render(*session, 0, 0, 1.0, 160, 120);
    QVERIFY(!initialPreview.isNull());
    QCOMPARE(qAlpha(initialPreview.pixel(5, 25)), 0);
    QVERIFY(qAlpha(initialPreview.pixel(25, 25)) > 0);
    const double pins[] = {30, 50, 30, 50, 0, 0, 80, 50, 100, 65, 0, 1};
    QCOMPARE(pictura::puppet_warp_configure(*session, ::rust::Slice<const double>(pins, 12),
                                            1, 1, 0), 2);
    QVERIFY(pictura::puppet_warp_commit(*view, *session));
    QCOMPARE(view->history_count(), history + 1);
    QVERIFY(view->image() != before);
    QVERIFY(view->undo());
    QCOMPARE(view->image(), before);
}

void PuppetWarpTest::autoRotationGesture()
{
    bool opened = false;
    QTimer::singleShot(0, [&] {
        auto* d = dialog();
        QVERIFY(d);
        const auto close = qScopeGuard([d] { if (d->isVisible()) d->reject(); });
        opened = true;
        auto* canvas = d->findChild<QWidget*>(QStringLiteral("puppetWarpCanvas"));
        auto* angle = d->findChild<QDoubleSpinBox*>(QStringLiteral("puppetWarpAngle"));
        QVERIFY(canvas && angle);
        deform(canvas);
        const QPoint ring = point(canvas, 125, 75) + QPoint(21, 0);
        drag(canvas, ring, ring + QPoint(0, 1), Qt::AltModifier);
        QVERIFY(angle->isEnabled());
        QVERIFY(qAbs(angle->value()) > 5.0);
        d->reject();
    });
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(kCommand)));
    QVERIFY(opened);
}

void PuppetWarpTest::foldPreviewCommit()
{
    auto* view = window_->activeView();
    auto session = pictura::puppet_warp_begin(*view);
    const double pins[] = {20, 30, 130, 30, 0, 0, 130, 30, 20, 30, 0, 1,
                           20, 100, 20, 100, 0, 2, 130, 100, 130, 100, 0, 3};
    const double reordered[] = {130, 30, 20, 30, 0, 1, 20, 30, 130, 30, 0, 0,
                                20, 100, 20, 100, 0, 2, 130, 100, 130, 100, 0, 3};
    QCOMPARE(pictura::puppet_warp_configure(*session, ::rust::Slice<const double>(pins, 24),
                                            1, 1, 0), 2);
    auto bounds = pictura::puppet_warp_bounds(*session);
    QCOMPARE(bounds.size(), size_t(4));
    const QImage first = pictura::puppet_warp_render(*session, bounds[0], bounds[1], 1.0,
                                                     bounds[2], bounds[3]);
    QVERIFY(!first.isNull());
    QCOMPARE(first, pictura::puppet_warp_render(*session, bounds[0], bounds[1], 1.0,
                                                bounds[2], bounds[3]));
    QCOMPARE(pictura::puppet_warp_configure(*session, ::rust::Slice<const double>(reordered, 24),
                                            1, 1, 0), 2);
    bounds = pictura::puppet_warp_bounds(*session);
    const QImage front = pictura::puppet_warp_render(*session, bounds[0], bounds[1], 1.0,
                                                     bounds[2], bounds[3]);
    QVERIFY(front != first);
    QVERIFY(pictura::puppet_warp_commit(*view, *session));
    QImage expected(160, 120, QImage::Format_ARGB32_Premultiplied);
    expected.fill(Qt::white);
    QPainter painter(&expected);
    painter.drawImage(QPoint(bounds[0], bounds[1]), front);
    painter.end();
    QCOMPARE(view->image().convertToFormat(expected.format()), expected);
}

void PuppetWarpTest::samplingPreviewCommit()
{
    auto* view = window_->activeView();
    auto session = pictura::puppet_warp_begin(*view);
    const double nan = std::numeric_limits<double>::quiet_NaN();
    const double pins[] = {30, 30, 30, 30, nan, 0, 110, 80, 121, 87, nan, 1};
    QCOMPARE(pictura::puppet_warp_configure(*session, ::rust::Slice<const double>(pins, 12),
                                            1, 1, 10), 2);
    const auto bounds = pictura::puppet_warp_bounds(*session);
    QCOMPARE(bounds.size(), size_t(4));
    const QImage preview = pictura::puppet_warp_render(*session, bounds[0], bounds[1], 1.0,
                                                       bounds[2], bounds[3]);
    QVERIFY(!preview.isNull());
    QVERIFY(pictura::puppet_warp_commit(*view, *session));
    QImage expected(160, 120, QImage::Format_ARGB32_Premultiplied);
    expected.fill(Qt::white);
    QPainter painter(&expected);
    painter.drawImage(QPoint(bounds[0], bounds[1]), preview);
    painter.end();
    QCOMPARE(view->image().convertToFormat(expected.format()), expected);
}

PICTURA_QT_TEST_MAIN(PuppetWarpTest)
#include "tst_puppet_warp.moc"
