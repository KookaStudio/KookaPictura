#include "api/engine_bridge.h"
#include "api/layers_bridge.h"
#include "frame.h"
#include "commands.h"
#include "color_picker_dialog.h"
#include "panels/swatches_panel.h"
#include "image_view.h"
#include "panels/color_panel.h"
#include "panels/info_panel.h"
#include "panels/layers_panel.h"
#include "qt_test_support.h"

#include <QtCore/QDataStream>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtCore/QTimer>
#include <QtTest/QtTest>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QToolButton>

#include <algorithm>

namespace {

QDialog* visibleDialog(const char* name)
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        auto* dialog = qobject_cast<QDialog*>(widget);
        if (dialog && dialog->isVisible() && dialog->objectName() == QLatin1String(name)) {
            return dialog;
        }
    }
    return nullptr;
}

void armWatchdog(QTimer& timer)
{
    timer.setSingleShot(true);
    QObject::connect(&timer, &QTimer::timeout, [] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* dialog = qobject_cast<QDialog*>(widget); dialog && dialog->isVisible()) {
                dialog->reject();
            }
        }
    });
    timer.start(3000);
}

// One pixel, one named layer, raw Float32 RGB and partial straight alpha.
bool writeFixture(const QString& path, float mergedRed = 4.0f)
{
    const auto writeSamples = [mergedRed](QDataStream& stream) {
        for (const float sample : {mergedRed, 0.5f, 0.25f, 0.5f}) {
            stream << sample;
        }
    };
    QByteArray layers;
    QDataStream layer(&layers, QIODevice::WriteOnly);
    layer.setFloatingPointPrecision(QDataStream::SinglePrecision);
    layer << qint16(-1) << qint32(0) << qint32(0) << qint32(1) << qint32(1) << quint16(4);
    for (const auto id : {0, 1, 2, -1}) {
        layer << qint16(id) << quint32(6);
    }
    layer.writeRawData("8BIMnorm", 8);
    layer << quint8(255) << quint8(0) << quint8(0) << quint8(0);
    layer << quint32(12) << quint32(0) << quint32(0);
    layer.writeRawData("\001H\000\000", 4);
    for (const float sample : {4.0f, 0.5f, 0.25f, 0.5f}) {
        layer << quint16(0) << sample;
    }

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    QDataStream out(&file);
    out.setFloatingPointPrecision(QDataStream::SinglePrecision);
    out.writeRawData("8BPS", 4);
    out << quint16(1) << quint32(0) << quint16(0) << quint16(4);
    out << quint32(1) << quint32(1) << quint16(32) << quint16(3);
    out << quint32(0) << quint32(0);
    out << quint32(layers.size() + 8) << quint32(layers.size());
    out.writeRawData(layers.constData(), layers.size());
    out << quint32(0) << quint16(0);
    writeSamples(out);
    return out.status() == QDataStream::Ok && file.flush();
}

} // namespace

class HdrPreviewTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void optionsPreviewAndCancel();
    void optionsSurviveFileRoundTrip();
    void statusExposureResets();
    void pickerRetainsSceneColor();
    void colorMutationsStayInSync();
    void lossySwatchRequiresConfirmation();
    void infoReadsNativeSamples();
    void secondWindowHasIndependentExposure();
    void mergedSourceAndLayerEdit();
    void maskTransitionsPreserveSceneColor();

private:
    void chooseColor(pictura::ColorPanel& panel, const char* entry, double initial, double value,
                     bool accept);
    pictura::test::ScopedStateHome state_;
    QTemporaryDir directory_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
};

void HdrPreviewTest::init()
{
    QVERIFY(state_.isValid());
    QVERIFY(directory_.isValid());
    window_ = pictura::test::makeMainWindow();
    window_->show();
    const QString path = directory_.filePath(QStringLiteral("layered.psd"));
    QVERIFY(writeFixture(path));
    QVERIFY(window_->openPath(path));
    view_ = window_->activeView();
    QVERIFY(view_);
    QCOMPARE(view_->document_depth_bits(), 32);
}

void HdrPreviewTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    window_.reset();
}

void HdrPreviewTest::optionsPreviewAndCancel()
{
    const auto before = view_->canvas_display_image(0, 0, 0, 1, 1);
    const auto original = pictura::hdr_scene_pixel(*view_, 0, 0);
    QCOMPARE(original.size(), size_t(4));
    QCOMPARE(original[0], 4.0);
    QCOMPARE(original[3], 0.5);
    const auto menuActions = window_->findChildren<QAction*>();
    auto action = std::find_if(menuActions.begin(), menuActions.end(), [](QAction* item) {
        return item->text().contains(QStringLiteral("32-Bit Preview Options"));
    });
    QVERIFY(action != menuActions.end());
    bool visited = false;
    QTimer watchdog;
    armWatchdog(watchdog);
    QTimer::singleShot(0, this, [&] {
        auto* dialog = visibleDialog("hdrPreviewDialog");
        if (!dialog) {
            return;
        }
        auto* method = dialog->findChild<QComboBox*>(QStringLiteral("hdrPreviewMethod"));
        auto* exposure = dialog->findChild<QDoubleSpinBox*>(QStringLiteral("hdrPreviewExposure"));
        if (!method || !exposure) {
            dialog->reject();
            return;
        }
        exposure->setValue(-1.0);
        const QImage mapped = view_->canvas_display_image(0, 0, 0, 1, 1);
        method->setCurrentIndex(1);
        visited = !mapped.isNull() && mapped != before
            && view_->canvas_display_image(0, 0, 0, 1, 1) != mapped
            && pictura::hdr_scene_pixel(*view_, 0, 0)[0] == original[0];
        dialog->reject();
    });
    (*action)->trigger();
    QVERIFY(visited);
    QCOMPARE(view_->canvas_display_image(0, 0, 0, 1, 1), before);
}

void HdrPreviewTest::optionsSurviveFileRoundTrip()
{
    QVERIFY(pictura::set_hdr_preview_options(*view_, 1, -2.0, 1.6, 1));
    window_->imageView()->setHdrExposure(32);
    for (const QString& extension : {QStringLiteral("psd"), QStringLiteral("psb")}) {
        const QString path = directory_.filePath(QStringLiteral("saved.") + extension);
        QVERIFY(view_->save(path));
        QVERIFY(window_->openPath(path));
        auto* reopened = window_->activeView();
        const auto params = pictura::hdr_preview_options(*reopened);
        QCOMPARE(params.size(), size_t(3));
        QCOMPARE(params[0], 1.0);
        QCOMPARE(params[1], -2.0);
        QVERIFY(qAbs(params[2] - 1.6) < 1e-5);
        QCOMPARE(window_->imageView()->hdrExposure(), 0);
        QCOMPARE(pictura::hdr_scene_pixel(*reopened, 0, 0)[0], 4.0);
    }
}

void HdrPreviewTest::statusExposureResets()
{
    auto* button = window_->findChild<QToolButton*>(QStringLiteral("statusOptionsButton"));
    QVERIFY(button);
    const auto options = button->menu()->actions();
    auto exposure = std::find_if(options.begin(), options.end(), [](QAction* action) {
        return action->data().toString() == QStringLiteral("hdrExposure");
    });
    QVERIFY(exposure != options.end());
    (*exposure)->trigger();
    auto* slider = window_->findChild<QSlider*>(QStringLiteral("hdrExposureSlider"));
    QVERIFY(slider);
    auto* canvas = window_->imageView();
    const QImage normal = pictura::hdr_canvas_image(*view_, 0, 0, 0, 1, 1, 0);
    slider->setValue(-32);
    QCOMPARE(canvas->hdrExposure(), -32);
    QVERIFY(pictura::hdr_canvas_image(*view_, 0, 0, 0, 1, 1, -32) != normal);
    QTest::mouseDClick(slider, Qt::LeftButton);
    QCOMPARE(canvas->hdrExposure(), 0);
}

void HdrPreviewTest::chooseColor(pictura::ColorPanel& panel, const char* entry,
                                 double initial, double value, bool accept)
{
    auto* button = panel.findChild<QPushButton*>(QString::fromLatin1(entry));
    QVERIFY(button);
    double read = -1.0;
    QTimer watchdog;
    armWatchdog(watchdog);
    QTimer::singleShot(0, this, [&] {
        if (auto* dialog = visibleDialog("hdrPickerDialog")) {
            if (auto* red = dialog->findChild<QDoubleSpinBox*>(QStringLiteral("hdrPickerRed"))) {
                read = red->value();
                red->setValue(value);
            }
            accept ? dialog->accept() : dialog->reject();
        }
    });
    button->click();
    QCOMPARE(read, initial);
}

void HdrPreviewTest::pickerRetainsSceneColor()
{
    auto* panel = window_->findChild<pictura::ColorPanel*>();
    QVERIFY(panel);
    chooseColor(*panel, "colorPanelForeground", 0.0, 4.0, true);
    chooseColor(*panel, "colorPanelBackground", 1.0, 2.0, true);
    chooseColor(*panel, "colorPanelForeground", 4.0, 8.0, false);
    chooseColor(*panel, "colorPanelForeground", 4.0, 0.0, false);
    chooseColor(*panel, "colorPanelBackground", 2.0, 0.0, false);
}

void HdrPreviewTest::colorMutationsStayInSync()
{
    auto* state = window_->findChild<pictura::ColorState*>();
    QVERIFY(state);
    state->setSceneColor(true, {4.0, 0.5, 0.25});
    state->setSceneColor(false, {2.0, 0.8, 0.125});
    state->swapColors();
    QCOMPARE(state->sceneColor(true)[0], 2.0);
    QCOMPARE(state->sceneColor(false)[0], 4.0);
    state->setForeground(QColor(Qt::black));
    state->setBackground(QColor(Qt::white));
    QCOMPARE(state->sceneColor(true)[0], 0.0);
    QCOMPARE(state->sceneColor(false)[0], 1.0);
    state->setSceneColor(true, {2.0, 0.0, 0.0});
    state->setForeground(QColor(Qt::red));
    QCOMPARE(state->sceneColor(true)[0], 1.0);
}

void HdrPreviewTest::lossySwatchRequiresConfirmation()
{
    auto* panel = window_->findChild<pictura::ColorPanel*>();
    auto* grid = pictura::ColorPickerDialog::defaultSwatchGrid();
    QVERIFY(panel && grid);
    auto* button = panel->findChild<QPushButton*>(QStringLiteral("colorPanelForeground"));
    QVERIFY(button);
    const auto before = grid->swatches().size();
    bool cancelled = false;
    bool accepted = false;
    QTimer watchdog;
    armWatchdog(watchdog);
    QTimer::singleShot(0, this, [&] {
        auto* dialog = visibleDialog("hdrPickerDialog");
        if (!dialog) return;
        auto* red = dialog->findChild<QDoubleSpinBox*>(QStringLiteral("hdrPickerRed"));
        auto* add = dialog->findChild<QPushButton*>(QStringLiteral("hdrPickerAddSwatch"));
        if (red && add && add->isEnabled()) {
            red->setValue(4.0);
            QTimer::singleShot(0, dialog, [&] {
                auto* warning = dialog->findChild<QMessageBox*>();
                if (warning && warning->isVisible()) {
                    if (auto* response = warning->button(QMessageBox::Cancel)) {
                        cancelled = true;
                        response->click();
                    }
                }
            });
            add->click();
            cancelled = cancelled && grid->swatches().size() == before;
            QTimer::singleShot(0, dialog, [&] {
                auto* warning = dialog->findChild<QMessageBox*>();
                if (warning && warning->isVisible()) {
                    if (auto* response = warning->button(QMessageBox::Ok)) {
                        accepted = true;
                        response->click();
                    }
                }
            });
            add->click();
            accepted = accepted && grid->swatches().size() == before + 1;
        }
        dialog->reject();
    });
    button->click();
    QVERIFY(cancelled);
    QVERIFY(accepted);
}

void HdrPreviewTest::secondWindowHasIndependentExposure()
{
    QVERIFY(window_->registry()->dispatch(QStringLiteral("window.arrange.new.window.for.file")));
    QCOMPARE(window_->windowCountForTest(0), 2);
    auto* first = window_->windowCanvasForTest(0, 0);
    auto* second = window_->windowCanvasForTest(0, 1);
    QVERIFY(first && second && first != second);
    first->setHdrExposure(32);
    QCOMPARE(second->hdrExposure(), 0);
    auto* button = window_->findChild<QToolButton*>(QStringLiteral("statusOptionsButton"));
    QVERIFY(button && button->menu());
    const auto actions = button->menu()->actions();
    auto it = std::find_if(actions.begin(), actions.end(), [](QAction* action) {
        return action->data().toString() == QStringLiteral("hdrExposure");
    });
    QVERIFY(it != actions.end());
    (*it)->trigger();
    auto* slider = window_->findChild<QSlider*>(QStringLiteral("hdrExposureSlider"));
    QVERIFY(slider);
    slider->setValue(-32);
    QCOMPARE(first->hdrExposure(), 32);
    QCOMPARE(second->hdrExposure(), -32);
    QTest::mouseDClick(slider, Qt::LeftButton);
    QCOMPARE(second->hdrExposure(), 0);
    QCOMPARE(first->hdrExposure(), 32);
}

void HdrPreviewTest::mergedSourceAndLayerEdit()
{
    const QString path = directory_.filePath(QStringLiteral("different.psd"));
    QVERIFY(writeFixture(path, 2.0f));
    QVERIFY(window_->openPath(path));
    auto* view = window_->activeView();
    QVERIFY(view);
    QCOMPARE(pictura::hdr_scene_pixel(*view, 0, 0)[0], 2.0);
    QVERIFY(pictura::set_hdr_preview_options(*view, 0, -1.0, 1.0, 1));
    const QString saved = directory_.filePath(QStringLiteral("different-saved.psd"));
    QVERIFY(view->save(saved));
    QVERIFY(window_->openPath(saved));
    view = window_->activeView();
    QVERIFY(view);
    QCOMPARE(pictura::hdr_scene_pixel(*view, 0, 0)[0], 2.0);
    const QImage before = view->canvas_display_image(0, 0, 0, 1, 1);
    QVERIFY(view->preview_layers_opacity(QStringList{QStringLiteral("0")}, 128) > 0);
    QCOMPARE(pictura::hdr_scene_pixel(*view, 0, 0)[0], 4.0);
    QVERIFY(view->canvas_display_image(0, 0, 0, 1, 1) != before);
    QVERIFY(view->commit_layers_opacity(QStringList{QStringLiteral("0")}, 128) > 0);
    QCOMPARE(pictura::hdr_scene_pixel(*view, 0, 0)[0], 4.0);
    QVERIFY(window_->registry()->dispatch(QStringLiteral("edit.undo")));
    QCOMPARE(pictura::hdr_scene_pixel(*view, 0, 0)[0], 2.0);
}

void HdrPreviewTest::maskTransitionsPreserveSceneColor()
{
    auto* colors = window_->findChild<pictura::ColorState*>();
    QVERIFY(colors);
    colors->setSceneColor(true, {4.0, 0.5, 0.25});
    colors->setSceneColor(false, {2.0, 0.25, 0.5});
    QVERIFY(pictura::quick_mask_set(*view_, true));
    QCoreApplication::processEvents();
    QVERIFY(pictura::quick_mask_set(*view_, false));
    QCoreApplication::processEvents();
    QCOMPARE(colors->sceneColor(true)[0], 4.0);
    QCOMPARE(colors->sceneColor(false)[0], 2.0);
    auto* layers = window_->findChild<pictura::LayersPanel*>();
    QVERIFY(layers);
    QVERIFY(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")));
    QCoreApplication::processEvents();
    const QString path = QStringLiteral("0");
    QVERIFY(layers->clickMaskThumbnailForTest(path));
    QVERIFY(colors->maskActive());
    QVERIFY(layers->clickLayerThumbnailForTest(path));
    QVERIFY(!colors->maskActive());
    QCOMPARE(colors->sceneColor(true)[0], 4.0);
    QCOMPARE(colors->sceneColor(false)[0], 2.0);
}

void HdrPreviewTest::infoReadsNativeSamples()
{
    auto* panel = window_->findChild<pictura::InfoPanel*>();
    QVERIFY(panel);
    panel->setView(view_);
    panel->setBitDepthForTest(0, 32);
    panel->setCursorPosition(QPointF(0, 0));
    const QString before = panel->colorBlockTextForTest(0);
    QVERIFY(before.contains(QStringLiteral("R : 4")));
    window_->imageView()->setHdrExposure(-32);
    panel->setCursorPosition(QPointF(0.1, 0.1));
    QCOMPARE(panel->colorBlockTextForTest(0), before);
}

PICTURA_QT_TEST_MAIN(HdrPreviewTest)
#include "tst_hdr_preview.moc"
