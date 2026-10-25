// The fill layer editors: the bridge write/read path, the dialog payloads, the
// Pattern gating, and the content-thumbnail dispatch (#895).

#include <QtTest/QSignalSpy>
#include <QtTest/QtTest>

#include "api/layers_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/fill_content.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"

#include "color_picker_dialog.h"
#include "fill_layer_editor.h"
#include "frame.h"
#include "gradient_fill_dialog.h"
#include "layer_new_dialog.h"
#include "panels/adjustments_panel.h"
#include "panels/layers_panel.h"
#include "pattern_fill_dialog.h"
#include "qt_test_support.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>

#include "commands.h"
#include "panels/numeric_field.h"

namespace {

QString patternFixture()
{
    QString fixture =
        QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/stroke_pattern.psd");
    QDir dir(QCoreApplication::applicationDirPath());
    for (int i = 0; i < 6 && fixture.isEmpty(); ++i) {
        const QString candidate =
            dir.filePath(QStringLiteral("crates/pictura-codec/tests/fixtures/stroke_pattern.psd"));
        if (QFileInfo::exists(candidate)) {
            fixture = candidate;
        } else if (!dir.cdUp()) {
            break;
        }
    }
    return fixture;
}

void rejectNextModal(bool* seen = nullptr)
{
    QTimer::singleShot(0, [seen] {
        auto* dialog = qobject_cast<QDialog*>(QApplication::activeModalWidget());
        if (!dialog) {
            // runDialog presents a non-modal dialog with its own event loop.
            for (QWidget* widget : QApplication::topLevelWidgets()) {
                auto* candidate = qobject_cast<QDialog*>(widget);
                if (candidate && candidate->isVisible()) {
                    dialog = candidate;
                    break;
                }
            }
        }
        if (dialog) {
            if (seen) {
                *seen = true;
            }
            dialog->reject();
        }
    });
}

} // namespace

class FillLayerEditorTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void solidApply();
    void solidCancel();
    void gradientApply();
    void gradientEditorOpens();
    void gradientCancel();
    void gradientPersistence();
    void patternApply();
    void patternSnapToOrigin();
    void patternEmpty();
    void layerFillMenuPatternGating();
    void patternIconGating();
    void createFromLayerMenu();
    void createCancel();
    void createCancelAfterTypeEditor();
    void createFromPanel();
    void contentOptionsOpensEditor();
    void thumbDoubleClickOpensEditor();
    void thumbDoubleClickUsesClickedPath();
    void adjustmentThumbOpensProperties();
    void maskThumbKeepsTarget();
    void backgroundDoubleClickConverts();

private:
    bool newDoc(const QString& name);
    bool openPatternFixture();
    QString addFill(const QString& kind, const QString& payload);
    pictura::layers_api::Snapshot snapshot() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
};

void FillLayerEditorTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(900, 600);
    window_->show();
    QCoreApplication::processEvents();
}

void FillLayerEditorTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
}

bool FillLayerEditorTest::newDoc(const QString& name)
{
    if (!window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    return view_ && panel_;
}

bool FillLayerEditorTest::openPatternFixture()
{
    const QString fixture = patternFixture();
    if (fixture.isEmpty() || !window_->openDocumentAtPath(fixture)) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    return view_ && panel_ && layer_has_patterns(*view_);
}

QString FillLayerEditorTest::addFill(const QString& kind, const QString& payload)
{
    return pictura::add_fill_layer(*view_, kind, payload, layer_selection_anchor(*view_),
                                   QString(), 0, QStringLiteral("norm"), 255, false);
}

pictura::layers_api::Snapshot FillLayerEditorTest::snapshot() const
{
    return pictura::layers_api::snapshot(*view_, 24, false);
}

void FillLayerEditorTest::solidApply()
{
    QVERIFY(newDoc(QStringLiteral("SolidApply")));
    const QString path = view_->add_solid_fill(0xff000000u);
    QVERIFY(!path.isEmpty());
    QVERIFY(set_layer_fill_color(*view_, path, 0xff112233u));
    QCOMPARE(layer_fill_kind(*view_, path), QStringLiteral("solid-color"));
    QCOMPARE(layer_fill_color(*view_, path), 0xff112233u);
}

void FillLayerEditorTest::solidCancel()
{
    rejectNextModal();
    const QColor color = pictura::ColorPickerDialog::getColor(Qt::red);
    QVERIFY(!color.isValid());
}

void FillLayerEditorTest::gradientApply()
{
    QVERIFY(newDoc(QStringLiteral("GradientApply")));
    const QString path = view_->add_gradient_fill();
    QVERIFY(!path.isEmpty());
    pictura::GradientFillDialog dialog(
        QStringLiteral("linear|0|100|0|0:000000,4096:ffffff"));
    dialog.styleForTest()->setCurrentIndex(
        dialog.styleForTest()->findData(QStringLiteral("radial")));
    dialog.angleForTest()->setValue(45);
    dialog.scaleForTest()->setValue(60);
    dialog.reverseForTest()->setChecked(true);
    const QString payload = dialog.payload();
    QVERIFY(payload.startsWith(QStringLiteral("radial|45|60|1|")));
    QVERIFY(set_layer_fill_gradient(*view_, path, payload));
    QVERIFY(layer_fill_gradient(*view_, path).startsWith(QStringLiteral("radial|45|60|1|")));
}

void FillLayerEditorTest::gradientEditorOpens()
{
    pictura::GradientFillDialog dialog(
        QStringLiteral("linear|0|100|0|0:000000,4096:ffffff"));
    rejectNextModal();
    dialog.openEditor();
    QVERIFY(true);
}

void FillLayerEditorTest::gradientCancel()
{
    rejectNextModal();
    QString payload;
    QVERIFY(!pictura::GradientFillDialog::getFill(
        QStringLiteral("linear|0|100|0|0:000000,4096:ffffff"), nullptr, &payload));
}

void FillLayerEditorTest::gradientPersistence()
{
    QVERIFY(newDoc(QStringLiteral("GradientPersist")));
    const QString path = view_->add_gradient_fill();
    QVERIFY(!path.isEmpty());
    const QString payload = QStringLiteral("diamond|12.5|60|1|0:000000,4096:ffffff");
    QVERIFY(set_layer_fill_gradient(*view_, path, payload));
    const QString read = layer_fill_gradient(*view_, path);
    QVERIFY(read.startsWith(QStringLiteral("diamond|12.5|60|1|")));
}

void FillLayerEditorTest::patternApply()
{
    QVERIFY2(openPatternFixture(), "the pattern fixture must carry patterns");
    const QStringList ids = document_pattern_ids(*view_);
    QVERIFY(!ids.isEmpty());
    const QString path = addFill(QStringLiteral("pattern"),
                                 ids.first() + QStringLiteral("|50|1|0|0"));
    QVERIFY(!path.isEmpty());
    QCOMPARE(layer_fill_kind(*view_, path), QStringLiteral("pattern"));
    QVERIFY(layer_fill_pattern(*view_, path).startsWith(ids.first() + QStringLiteral("|50|1|")));
}

void FillLayerEditorTest::patternSnapToOrigin()
{
    QVERIFY2(openPatternFixture(), "the pattern fixture must carry patterns");
    const QStringList ids = document_pattern_ids(*view_);
    QVERIFY(!ids.isEmpty());
    pictura::PatternFillDialog dialog(*view_, ids.first() + QStringLiteral("|50|1|3|4"));
    QVERIFY(dialog.snapForTest());
    dialog.snapForTest()->click();
    QVERIFY(dialog.payload().endsWith(QStringLiteral("|0|0")));
}

void FillLayerEditorTest::patternEmpty()
{
    QVERIFY(newDoc(QStringLiteral("NoPatterns")));
    pictura::PatternFillDialog dialog(*view_, QString());
    QVERIFY(dialog.emptyForTest());
}

void FillLayerEditorTest::layerFillMenuPatternGating()
{
    QVERIFY(newDoc(QStringLiteral("PatternGate")));
    QVERIFY(!window_->registry()->action(QStringLiteral("layer.new.fill.layer.pattern"))->isEnabled());
    QVERIFY(window_->openDocumentAtPath(patternFixture()));
    view_ = window_->activeView();
    QVERIFY(view_ && layer_has_patterns(*view_));
    QVERIFY(window_->registry()->action(QStringLiteral("layer.new.fill.layer.pattern"))->isEnabled());
}

void FillLayerEditorTest::patternIconGating()
{
    QVERIFY(newDoc(QStringLiteral("PatternIcon")));
    auto* adjustments =
        window_->findChild<pictura::AdjustmentsPanel*>(QStringLiteral("adjustmentsPanel"));
    QVERIFY(adjustments);
    QVERIFY(!adjustments->buttonByIdForTest(QStringLiteral("pattern"))->isEnabled());
    QVERIFY(window_->openDocumentAtPath(patternFixture()));
    view_ = window_->activeView();
    QVERIFY(view_ && layer_has_patterns(*view_));
    adjustments->setView(view_);
    QVERIFY(adjustments->buttonByIdForTest(QStringLiteral("pattern"))->isEnabled());
}

void FillLayerEditorTest::createFromLayerMenu()
{
    QVERIFY(newDoc(QStringLiteral("CreateMenu")));
    const QString a = view_->add_layer_in(QString());
    const QString b = view_->add_layer_in(QString());
    QVERIFY(!a.isEmpty() && !b.isEmpty());
    pictura::select_layer_path(*view_, a);
    const int before = pictura::layers_api::rowCount(*view_);
    const int history = view_->history_count();
    // Accept the New Layer dialog, then the Gradient Fill editor.
    QTimer timer;
    QObject::connect(&timer, &QTimer::timeout, [] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            auto* dialog = qobject_cast<QDialog*>(widget);
            if (dialog && dialog->isVisible()) {
                dialog->accept();
                return;
            }
        }
    });
    timer.start(10);
    QVERIFY(pictura::createFillLayerFromDialog(*view_, QStringLiteral("gradient"), Qt::red,
                                               nullptr, true));
    timer.stop();
    QCOMPARE(pictura::layers_api::rowCount(*view_), before + 1);
    QCOMPARE(view_->history_count(), history + 1);
    const QString created = pictura::selected_layer_paths(*view_).last();
    QCOMPARE(created, QStringLiteral("2"));
    QCOMPARE(pictura::layers_api::rowForPath(*view_, created).kind,
             QStringLiteral("adjustment"));
}

void FillLayerEditorTest::createCancelAfterTypeEditor()
{
    QVERIFY(newDoc(QStringLiteral("CreateCancelType")));
    const int before = pictura::layers_api::rowCount(*view_);
    const int history = view_->history_count();
    // Accept the New Layer dialog, then cancel the type editor.
    int seen = 0;
    QTimer timer;
    QObject::connect(&timer, &QTimer::timeout, [&seen] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            auto* dialog = qobject_cast<QDialog*>(widget);
            if (dialog && dialog->isVisible()) {
                if (seen++ == 0) {
                    dialog->accept();
                } else {
                    dialog->reject();
                }
                return;
            }
        }
    });
    timer.start(10);
    QVERIFY(!pictura::createFillLayerFromDialog(*view_, QStringLiteral("gradient"), Qt::red,
                                                nullptr, true));
    timer.stop();
    QCOMPARE(pictura::layers_api::rowCount(*view_), before);
    QCOMPARE(view_->history_count(), history);
}

void FillLayerEditorTest::createCancel()
{
    QVERIFY(newDoc(QStringLiteral("CreateCancel")));
    const int before = pictura::layers_api::rowCount(*view_);
    rejectNextModal();
    pictura::LayerNewSpec spec;
    QVERIFY(!pictura::LayerNewDialog::get(false, nullptr, &spec));
    QCOMPARE(pictura::layers_api::rowCount(*view_), before);
}

void FillLayerEditorTest::createFromPanel()
{
    QVERIFY(newDoc(QStringLiteral("CreatePanel")));
    const int before = pictura::layers_api::rowCount(*view_);
    rejectNextModal();
    QVERIFY(pictura::createFillLayerFromDialog(*view_, QStringLiteral("solid-color"), Qt::red,
                                               nullptr, false));
    QCOMPARE(pictura::layers_api::rowCount(*view_), before + 1);
}

void FillLayerEditorTest::contentOptionsOpensEditor()
{
    QVERIFY(newDoc(QStringLiteral("ContentOptions")));
    const QString path = view_->add_solid_fill(0xff000000u);
    QVERIFY(!path.isEmpty());
    pictura::select_layer_path(*view_, path);
    bool seen = false;
    rejectNextModal(&seen);
    QVERIFY(window_->registry()->dispatch(QStringLiteral("layer.contentOptions")));
    QVERIFY2(seen, "the Solid Color editor opens from Layer Content Options");
}

void FillLayerEditorTest::thumbDoubleClickOpensEditor()
{
    QVERIFY(newDoc(QStringLiteral("ThumbEdit")));
    const QString path = view_->add_solid_fill(0xff000000u);
    QVERIFY(!path.isEmpty());
    panel_->refresh();
    bool seen = false;
    rejectNextModal(&seen);
    QVERIFY(panel_->doubleClickThumbnailForTest(path));
    QVERIFY2(seen, "the Solid Color editor opens from the content thumbnail");
}

void FillLayerEditorTest::thumbDoubleClickUsesClickedPath()
{
    QVERIFY(newDoc(QStringLiteral("ThumbClicked")));
    const QString fill = view_->add_solid_fill(0xff000000u);
    QVERIFY(!fill.isEmpty());
    QVERIFY(!view_->add_layer_in(QString()).isEmpty());
    panel_->refresh();
    bool seen = false;
    rejectNextModal(&seen);
    QVERIFY(panel_->doubleClickThumbnailForTest(fill));
    QVERIFY2(seen, "the clicked fill, not the current layer, opens its editor");
}

void FillLayerEditorTest::adjustmentThumbOpensProperties()
{
    QVERIFY(newDoc(QStringLiteral("ThumbAdjustment")));
    QVERIFY(view_->add_adjustment(QStringLiteral("invert")));
    const QString path = pictura::selected_layer_paths(*view_).last();
    panel_->refresh();
    QSignalSpy requested(panel_, &pictura::LayersPanel::editContentRequested);
    QVERIFY(panel_->doubleClickThumbnailForTest(path));
    QCOMPARE(requested.count(), 1);
    QVERIFY2(window_->registry()->dispatch(QStringLiteral("layer.contentOptions")),
             "Layer Content Options runs for the adjustment");
}

void FillLayerEditorTest::maskThumbKeepsTarget()
{
    QVERIFY(newDoc(QStringLiteral("MaskThumb")));
    view_->deselect();
    const QString path = view_->add_solid_fill(0xff000000u);
    QVERIFY(!path.isEmpty());
    QVERIFY(pictura::layers_api::rowForPath(*view_, path).hasMask);
    panel_->refresh();
    QVERIFY(panel_->clickMaskThumbnailForTest(path));
    QCOMPARE(snapshot().maskTargetPath, path);
    QVERIFY(panel_->doubleClickMaskForTest(path));
    QCOMPARE(snapshot().maskTargetPath, path);
}

void FillLayerEditorTest::backgroundDoubleClickConverts()
{
    QVERIFY(newDoc(QStringLiteral("BackgroundDouble")));
    rejectNextModal();
    panel_->doubleClickAtForTest(QStringLiteral("0"), false);
    QVERIFY(true);
}

QTEST_MAIN(FillLayerEditorTest)
#include "tst_fill_layer_editor.moc"
