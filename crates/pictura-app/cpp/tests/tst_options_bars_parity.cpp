// Options-bars parity (#252): the tool-icon separator on every page, the Move
// bar's Auto-Select / Show Transform Controls / three-dots Align To, and the
// Eyedropper's Sample Size / Sample scope / Show Sampling Ring.

#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "options_bar.h"
#include "panels/layers_panel.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/eyedropper_sample.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QAction>
#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>

class OptionsBarsParityTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void toolIconSeparatorOnEveryPage();
    void moveControlsAndAlignToCanvas();
    void eyedropperSampleSize();
    void eyedropperSampleScope();
    void eyedropperSamplingRing();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;

    // The layer's rect offset as (left, top).
    QPoint offset(pictura::PictureView* view, const QString& path) const
    {
        const QStringList parts = view->layer_rect(path).split(QLatin1Char(' '));
        return parts.size() == 4 ? QPoint(parts[0].toInt(), parts[1].toInt()) : QPoint(-999, -999);
    }

    static QAction* menuAction(QToolButton* menuButton, const QString& objectName)
    {
        for (QAction* action : menuButton->menu()->actions()) {
            if (action->objectName() == objectName) {
                return action;
            }
        }
        return nullptr;
    }
};

void OptionsBarsParityTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void OptionsBarsParityTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

void OptionsBarsParityTest::toolIconSeparatorOnEveryPage()
{
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    for (pictura::ToolId id : pictura::allToolIds()) {
        QWidget* page = bar->pageForTest(id);
        QVERIFY(page);
        auto* layout = qobject_cast<QHBoxLayout*>(page->layout());
        QVERIFY(layout);
        auto* icon = qobject_cast<QToolButton*>(layout->itemAt(0)->widget());
        QVERIFY(icon);
        auto* separator = qobject_cast<QFrame*>(layout->itemAt(1)->widget());
        QVERIFY(separator);
        QCOMPARE(separator->objectName(), QStringLiteral("optionsToolSeparator"));
        QCOMPARE(separator->frameShape(), QFrame::VLine);
    }
}

void OptionsBarsParityTest::moveControlsAndAlignToCanvas()
{
    QVERIFY(window_->newDocument(QStringLiteral("Move"), 40, 40, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(view && panel && tools);

    view->select_rect(8, 5, 6, 6, QStringLiteral("new"), 0.0);
    const QString layer = view->layer_via_copy(QStringLiteral("0"));
    QVERIFY(!layer.isEmpty());
    view->deselect();
    QCoreApplication::processEvents();
    panel->selectPaths({layer}, layer);
    window_->setActiveTool(pictura::ToolId::Move);

    auto* autoSelect = window_->findChild<QComboBox*>(QStringLiteral("optionsMoveAutoSelect"));
    auto* transform = window_->findChild<QCheckBox*>(QStringLiteral("optionsMoveTransform"));
    auto* menuButton = window_->findChild<QToolButton*>(QStringLiteral("optionsMoveMenu"));
    QVERIFY(autoSelect && transform && menuButton && menuButton->menu());
    QCOMPARE(autoSelect->count(), 2);
    QVERIFY(menuAction(menuButton, QStringLiteral("moveMenuAlignLeft")));
    QVERIFY(menuAction(menuButton, QStringLiteral("moveMenuDistributeTop")));
    QVERIFY(menuAction(menuButton, QStringLiteral("moveMenuAlignToCanvas")));

    autoSelect->setCurrentIndex(1);
    QCOMPARE(tools->moveAutoSelect(), 1);
    transform->setChecked(true);
    QVERIFY(tools->moveShowTransformControls());

    // Align To Canvas: the square's content left (8) meets the canvas left (0),
    // so the document-sized copy shifts by -8.
    menuAction(menuButton, QStringLiteral("moveMenuAlignToCanvas"))->trigger();
    QCOMPARE(tools->moveAlignTo(), 1);
    menuAction(menuButton, QStringLiteral("moveMenuAlignLeft"))->trigger();
    QCOMPARE(offset(view, layer), QPoint(-8, 0));
}

void OptionsBarsParityTest::eyedropperSampleSize()
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(20, 0, 20, 40, Qt::black);
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_eyedropper_size"));
    QVERIFY(f.ok());

    // Point Sample on the black half.
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 30, 20, 1, 2), QRgb(0xFF000000));
    // A 5x5 average straddling the boundary is a mid grey, neither pure.
    const quint32 average = pictura::sample_argb_scoped(*f.view, 20, 20, 5, 2);
    QVERIFY(qRed(average) > 0 && qRed(average) < 255);

    // The handler's foreground follows the chosen size.
    f.tools->setForeground(QColor(1, 2, 3));
    f.tools->setActiveTool(pictura::ToolId::Eyedropper);
    auto* sizeCombo = window_->findChild<QComboBox*>(QStringLiteral("optionsEyedropperSize"));
    QVERIFY(sizeCombo);
    sizeCombo->setCurrentIndex(1); // 3 by 3 Average
    f.canvas->mousePressed(QPointF(20, 20), Qt::LeftButton, 0);
    QVERIFY(f.tools->foreground() != QColor(1, 2, 3));
}

void OptionsBarsParityTest::eyedropperSampleScope()
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_eyedropper_scope"));
    QVERIFY(f.ok());

    // An invert adjustment darkens the composite but not the source pixels.
    QVERIFY(f.view->add_adjustment(QStringLiteral("invert")));
    f.view->set_active_layer(QStringLiteral("0"));

    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 2), QRgb(0xFF000000)); // All Layers
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 3), QRgb(0xFFFFFFFF)); // All, no adjustments
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 0), QRgb(0xFFFFFFFF)); // Current Layer
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 1), QRgb(0xFFFFFFFF)); // Current & Below
    QCOMPARE(pictura::sample_argb_scoped(*f.view, 20, 20, 1, 4), QRgb(0xFFFFFFFF)); // Below, no adjustments

    auto* sampleCombo = window_->findChild<QComboBox*>(QStringLiteral("optionsEyedropperSample"));
    QVERIFY(sampleCombo);
    QCOMPARE(sampleCombo->count(), 5);
}

void OptionsBarsParityTest::eyedropperSamplingRing()
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_eyedropper_ring"));
    QVERIFY(f.ok());
    f.tools->setActiveTool(pictura::ToolId::Eyedropper);

    auto* ring = window_->findChild<QCheckBox*>(QStringLiteral("optionsEyedropperRing"));
    auto* sizeCombo = window_->findChild<QComboBox*>(QStringLiteral("optionsEyedropperSize"));
    QVERIFY(ring && sizeCombo);
    QVERIFY(!f.canvas->hasSamplingRingForTest());

    // Point Sample explicitly: the eyedropper size persists on the shared tool
    // controller, so an earlier case must not leak its 3 by 3 Average here.
    sizeCombo->setCurrentIndex(0);
    ring->setChecked(true);
    f.canvas->mouseMoved(QPointF(10, 10));
    QVERIFY(f.canvas->hasSamplingRingForTest());
    QCOMPARE(f.canvas->samplingRingDiameterForTest(), 1.0);

    sizeCombo->setCurrentIndex(2); // 5 by 5 Average
    f.canvas->mouseMoved(QPointF(10, 10));
    QCOMPARE(f.canvas->samplingRingDiameterForTest(), 5.0);

    ring->setChecked(false);
    QVERIFY(!f.canvas->hasSamplingRingForTest());
}

QTEST_MAIN(OptionsBarsParityTest)
#include "tst_options_bars_parity.moc"
