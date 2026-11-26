#include <QtTest/QtTest>

#include "api/paint_bridge.h"
#include "api/view_bridge.h"
#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "hdr_conversion_dialog.h"
#include "load_layers_dialog.h"
#include "panels/curve_widget.h"
#include "qt_test_support.h"

#include <QtCore/QFile>
#include <QtCore/QPointer>
#include <QtCore/QScopeGuard>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtGui/QPixmap>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QDoubleSpinBox>

#include <functional>
#include <memory>

namespace {

template <typename T>
T* control(QWidget& parent, const char* name)
{
    return parent.findChild<T*>(QString::fromLatin1(name));
}

QDialog* visibleDialog()
{
    if (auto* modal = qobject_cast<QDialog*>(QApplication::activeModalWidget())) {
        return modal;
    }
    for (auto* widget : QApplication::topLevelWidgets()) {
        auto* conversion = qobject_cast<pictura::HdrConversionDialog*>(widget);
        if (conversion && conversion->isVisible()) {
            return conversion;
        }
    }
    return nullptr;
}

} // namespace

class MergeToHdrTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void rejectsTwoSources();
    void rejectsDeepSources();
    void estimatesMissingExposure();
    void addsCleanSavedDocuments();
    void rejectsInvalidCurve();
    void alignmentFailureIsAtomic();
    void selectsGhostBase();
    void previewBrightness();
    void publishesOrCancels();
    void temporaryStackAndBackground();
    void adjustsFloatTone();
    void togglesSourcesAtTwo();
    void editsToneCurve();
    void loadsLegacyTonePreset();
    void budgetFailureIsAtomic();

private:
    enum class Stage { Picker, Exposure, Preview, Conversion, Warning };
    using Hook = std::function<void(Stage, QDialog&)>;
    bool run(const Hook& hook = {});
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir fixtures_;
    std::unique_ptr<pictura::PicturaMainWindow> frame_;
    QStringList paths_;
    int depth_ = 32;
    bool align_ = false;
};

void MergeToHdrTest::init()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(fixtures_.isValid());
    frame_ = pictura::test::makeMainWindow();
    QVERIFY(frame_);
    paths_.clear();
    depth_ = 32;
    align_ = false;
    // Only the base carries a usable sample. Its sRGB code maps to
    // approximately 0.303 linear radiance at the median bracket exposure.
    for (int value : {0, 149, 255}) {
        QImage image(8, 8, QImage::Format_RGB32);
        image.fill(QColor(value, value, value));
        const QString path = fixtures_.filePath(QStringLiteral("bracket-%1.png").arg(value));
        QVERIFY(image.save(path));
        paths_.append(path);
    }
}

bool MergeToHdrTest::run(const Hook& hook)
{
    QTimer driver;
    QTimer deadline;
    bool timedOut = false;
    deadline.setSingleShot(true);
    QObject::connect(&deadline, &QTimer::timeout, &driver, [&] {
        timedOut = true;
        QPointer<QDialog> dialog = visibleDialog();
        QPointer<pictura::HdrConversionDialog> conversion;
        for (auto* widget : QApplication::topLevelWidgets()) {
            auto* candidate = qobject_cast<pictura::HdrConversionDialog*>(widget);
            if (candidate && candidate->isVisible()) {
                conversion = candidate;
                break;
            }
        }
        if (dialog) {
            dialog->reject();
        }
        if (conversion && conversion->isVisible()) {
            conversion->reject();
        }
    });
    deadline.start(15000);
    driver.setInterval(1);
    QPointer<QDialog> previous;
    QObject::connect(&driver, &QTimer::timeout, &driver, [&] {
        auto* dialog = visibleDialog();
        if (timedOut) {
            if (dialog) {
                dialog->reject();
            }
            return;
        }
        if (!dialog || dialog == previous || dialog->objectName() == QStringLiteral("jobProgress")) {
            return;
        }
        previous = dialog;
        const auto closeIfStillOpen = qScopeGuard([&] {
            if (dialog->isVisible()) {
                dialog->reject();
            }
        });
        if (auto* picker = qobject_cast<pictura::LoadLayersDialog*>(dialog)) {
            if (hook) {
                hook(Stage::Picker, *dialog);
            }
            if (timedOut || !dialog->isVisible()) {
                return;
            }
            picker->addFiles(paths_);
            if (auto* alignment = control<QCheckBox>(*dialog, "loadLayersAlign")) {
                alignment->setChecked(align_);
            }
            if (auto* ok = control<QPushButton>(*dialog, "loadLayersOk"); ok && ok->isEnabled()) {
                ok->click();
            }
        } else if (dialog->objectName() == QStringLiteral("hdrMergeManualExposure")) {
            if (hook) {
                hook(Stage::Exposure, *dialog);
            }
            if (timedOut || !dialog->isVisible()) {
                return;
            }
            for (int i = 0; i < 3; ++i) {
                auto* entry = control<QLineEdit>(*dialog, qPrintable(QStringLiteral("hdrMergeEv%1").arg(i)));
                if (entry) {
                    entry->setText(QString::number(1 - i));
                }
            }
            if (auto* ok = control<QPushButton>(*dialog, "hdrMergeAcceptExposure"); ok && ok->isEnabled()) {
                ok->click();
            }
        } else if (dialog->objectName() == QStringLiteral("hdrMergePreview")) {
            auto* depth = control<QComboBox>(*dialog, "hdrMergeDepth");
            if (depth) {
                depth->setCurrentIndex(depth->findData(depth_));
            }
            if (hook) {
                hook(Stage::Preview, *dialog);
            }
            if (!timedOut && dialog->isVisible()) {
                if (auto* ok = control<QPushButton>(*dialog, "hdrMergeAccept"); ok && ok->isEnabled()) {
                    ok->click();
                }
            }
        } else if (auto* tone = qobject_cast<pictura::HdrConversionDialog*>(dialog)) {
            if (hook) {
                hook(Stage::Conversion, *dialog);
            }
            if (!timedOut && dialog->isVisible()) {
                if (auto* method = control<QComboBox>(*tone, "hdrConversionMethod")) {
                    method->setCurrentIndex(2);
                }
                if (auto* exposure = control<QDoubleSpinBox>(*tone, "hdrConversionExposure")) {
                    exposure->setValue(-1.0);
                }
                tone->accept();
            }
        } else if (qobject_cast<QMessageBox*>(dialog)) {
            if (hook) {
                hook(Stage::Warning, *dialog);
            }
            if (!timedOut) {
                dialog->accept();
            }
        }
    });
    driver.start();
    const bool dispatched = frame_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::FileAutomateMergeToHdrPro));
    return dispatched && !timedOut;
}

void MergeToHdrTest::rejectsTwoSources()
{
    bool pickerSeen = false;
    paths_.removeLast();
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage == Stage::Picker) {
            pickerSeen = true;
            auto& picker = static_cast<pictura::LoadLayersDialog&>(dialog);
            picker.addFiles(paths_);
            auto* ok = control<QPushButton>(dialog, "loadLayersOk");
            QVERIFY(ok);
            QVERIFY(!ok->isEnabled());
        }
    }));
    QVERIFY(pickerSeen);
    QCOMPARE(frame_->documentCount(), 0);
}

void MergeToHdrTest::rejectsDeepSources()
{
    QImage deep(8, 8, QImage::Format_RGBA64);
    deep.fill(Qt::white);
    const QString path = fixtures_.filePath(QStringLiteral("deep.png"));
    QVERIFY(deep.save(path));
    paths_[0] = path;
    bool reported = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage == Stage::Warning) {
            auto* warning = qobject_cast<QMessageBox*>(&dialog);
            QVERIFY(warning);
            QVERIFY(warning->text().contains(QStringLiteral("8 bits")));
            reported = true;
        }
    }));
    QVERIFY(reported);
    QCOMPARE(frame_->documentCount(), 0);
}

void MergeToHdrTest::estimatesMissingExposure()
{
    bool previewSeen = false;
    bool exposureSeen = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage == Stage::Exposure) {
            exposureSeen = true;
        }
        if (stage == Stage::Preview) {
            previewSeen = true;
            auto* base = control<QComboBox>(dialog, "hdrMergeBase");
            QVERIFY(base);
            QCOMPARE(base->count(), 3);
            QVERIFY(base->itemText(0).contains(QStringLiteral("EV100")));
            dialog.reject();
        }
    }));
    QVERIFY(previewSeen);
    QVERIFY(!exposureSeen);
    QCOMPARE(frame_->documentCount(), 0);
}

void MergeToHdrTest::addsCleanSavedDocuments()
{
    QVERIFY(frame_->openDocumentAtPath(paths_.at(0), false));
    QVERIFY(frame_->openDocumentAtPath(paths_.at(1), false));
    auto* dirty = frame_->activeView();
    QVERIFY(dirty);
    QVERIFY(dirty->duplicate_layer(0) >= 0);
    QVERIFY(frame_->isDocumentDirty(frame_->activeDocumentIndex()));
    auto* scratch = new pictura::PictureView(frame_.get());
    QVERIFY(scratch->open_image(paths_.at(2)));
    frame_->addDocument(scratch, {});
    bool found = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage == Stage::Picker) {
            auto* add = control<QPushButton>(dialog, "loadLayersAddOpen");
            QVERIFY(add && add->isEnabled());
            add->click();
            QCOMPARE(static_cast<pictura::LoadLayersDialog&>(dialog).paths(), QStringList{paths_.at(0)});
            found = true;
            dialog.reject();
        }
    }));
    QVERIFY(found);
    QCOMPARE(frame_->documentCount(), 3);
}

void MergeToHdrTest::rejectsInvalidCurve()
{
    const QString path = fixtures_.filePath(QStringLiteral("invalid.phdr-curve"));
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    QVERIFY(file.write(R"({"kind":"pictura-hdr-response","version":2,"exponents":[0,1,1]})") > 0);
    file.close();
    pictura::setOpenFileNameProviderForTest([&](QWidget*, const QString&, const QString&,
                                                const QStringList&, QString*) { return path; });
    const auto reset = qScopeGuard([] { pictura::setOpenFileNameProviderForTest({}); });
    bool rejected = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage == Stage::Preview) {
            auto* load = control<QPushButton>(dialog, "hdrMergeLoadCurve");
            QVERIFY(load);
            QTimer::singleShot(0, &dialog, [&] {
                auto* warning = qobject_cast<QMessageBox*>(QApplication::activeModalWidget());
                if (warning) {
                    rejected = true;
                    warning->accept();
                }
            });
            load->click();
            dialog.reject();
        }
    }));
    QVERIFY(rejected);
    QCOMPARE(frame_->documentCount(), 0);
}

void MergeToHdrTest::alignmentFailureIsAtomic()
{
    align_ = true;
    bool failed = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage == Stage::Preview) {
            auto* ok = control<QPushButton>(dialog, "hdrMergeAccept");
            auto* error = control<QLabel>(dialog, "hdrMergeError");
            QVERIFY(ok && !ok->isEnabled());
            QVERIFY(error && !error->text().isEmpty());
            failed = true;
        }
    }));
    QVERIFY(failed);
    QCOMPARE(frame_->documentCount(), 0);
}

void MergeToHdrTest::selectsGhostBase()
{
    bool changed = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage == Stage::Preview) {
            auto* ghosts = control<QCheckBox>(dialog, "hdrMergeGhost");
            auto* base = control<QComboBox>(dialog, "hdrMergeBase");
            QVERIFY(ghosts && base);
            ghosts->setChecked(true);
            base->setCurrentIndex(0);
            QVERIFY(base->isEnabled());
            QVERIFY(base->itemData(0, Qt::ForegroundRole).isValid());
            QVERIFY(!base->itemData(1, Qt::ForegroundRole).isValid());
            changed = true;
        }
    }));
    QVERIFY(changed);
}

void MergeToHdrTest::previewBrightness()
{
    QVERIFY(run());
    QVERIFY(frame_->activeView());
    const auto baseline = frame_->activeView()->composite_argb(1, 1);
    bool previewChanged = false;
    bool previewMatches = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage == Stage::Preview) {
            auto* brightness = control<QDoubleSpinBox>(dialog, "hdrMergeBrightness");
            auto* image = control<QLabel>(dialog, "hdrMergeImage");
            QVERIFY(brightness && image);
            const QImage before = image->pixmap().toImage();
            previewMatches = before.pixelColor(1, 1).red() == qRed(baseline);
            brightness->setValue(1.0);
            previewChanged = before != image->pixmap().toImage();
        }
    }));
    QVERIFY(previewChanged);
    QVERIFY(previewMatches);
    QCOMPARE(frame_->documentCount(), 2);
    QCOMPARE(frame_->activeView()->composite_argb(1, 1), baseline);
}

void MergeToHdrTest::publishesOrCancels()
{
    QVERIFY(run());
    QCOMPARE(frame_->documentCount(), 1);
    QCOMPARE(frame_->activeView()->document_depth_bits(), 32);
    QVERIFY(frame_->documentPath(0).isEmpty());
    QVERIFY(frame_->isDocumentDirty(0));
    for (Stage phase : {Stage::Picker, Stage::Preview}) {
        bool reached = false;
        QVERIFY(run([&](Stage stage, QDialog& dialog) {
            if (stage == phase) {
                reached = true;
                dialog.reject();
            }
        }));
        QVERIFY(reached);
        QCOMPARE(frame_->documentCount(), 1);
    }
}

void MergeToHdrTest::temporaryStackAndBackground()
{
    bool inspected = false;
    QVERIFY(run([&](Stage stage, QDialog&) {
        if (stage == Stage::Preview) {
            inspected = true;
            QVERIFY(frame_->activeView());
            QCOMPARE(frame_->activeView()->document_depth_bits(), 32);
            QCOMPARE(frame_->activeView()->layer_count(), 4);
            QCOMPARE(frame_->activeView()->layer_name(0), QStringLiteral("Merge to HDR Pro"));
        }
    }));
    QVERIFY(inspected);
    QCOMPARE(frame_->activeView()->layer_count(), 1);
    QCOMPARE(frame_->activeView()->layer_name(0), QStringLiteral("Background"));
    QCOMPARE(frame_->activeView()->document_depth_bits(), 32);
}

void MergeToHdrTest::adjustsFloatTone()
{
    bool changed = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage != Stage::Preview) { return; }
        auto* image = control<QLabel>(dialog, "hdrMergeImage");
        auto* exposure = control<QDoubleSpinBox>(dialog, "hdrExposure");
        auto* method = control<QComboBox>(dialog, "hdrMergeToneMethod");
        QVERIFY(image && exposure && method);
        const QImage before = image->pixmap().toImage();
        method->setCurrentIndex(1);
        exposure->setValue(-1.0);
        changed = before != image->pixmap().toImage();
    }));
    QVERIFY(changed);
    QCOMPARE(frame_->activeView()->document_depth_bits(), 32);
}

void MergeToHdrTest::togglesSourcesAtTwo()
{
    bool previewSeen = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage != Stage::Preview) { return; }
        previewSeen = true;
        QVERIFY(dialog.isFullScreen());
        auto* strip = control<QScrollArea>(dialog, "hdrMergeSources");
        auto* first = control<QCheckBox>(dialog, "hdrMergeSource0");
        auto* second = control<QCheckBox>(dialog, "hdrMergeSource1");
        auto* third = control<QCheckBox>(dialog, "hdrMergeSource2");
        auto* ghost = control<QCheckBox>(dialog, "hdrMergeGhost");
        auto* base = control<QComboBox>(dialog, "hdrMergeBase");
        auto* image = control<QLabel>(dialog, "hdrMergeImage");
        auto* accept = control<QPushButton>(dialog, "hdrMergeAccept");
        QVERIFY(strip && first && second && third && ghost && base && image && accept);
        QVERIFY(first->isChecked() && second->isChecked() && third->isChecked());
        ghost->setChecked(true);
        base->setCurrentIndex(0);
        first->click();
        QVERIFY(!first->isChecked() && first->isEnabled());
        QVERIFY(second->isChecked() && !second->isEnabled());
        QVERIFY(third->isChecked() && !third->isEnabled());
        second->click();
        QVERIFY(second->isChecked());
        QVERIFY(!frame_->activeView()->layer_visible(1));
        QVERIFY(base->currentIndex() != 0);
        QVERIFY(accept->isEnabled());
        first->click();
        QVERIFY(first->isChecked() && second->isEnabled() && third->isEnabled());
        QCOMPARE(base->currentIndex(), 0);
        QVERIFY(frame_->activeView()->layer_visible(1));
        ghost->setChecked(false);
        const QImage full = image->pixmap().toImage();
        second->click();
        QVERIFY(!image->pixmap().isNull());
        QVERIFY(full != image->pixmap().toImage());
        QTest::keyClick(&dialog, Qt::Key_Escape);
    }));
    QVERIFY(previewSeen);
    QCOMPARE(frame_->documentCount(), 0);

    QVERIFY(run());
    const auto full = frame_->activeView()->composite_argb(1, 1);
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage == Stage::Preview) {
            auto* middle = control<QCheckBox>(dialog, "hdrMergeSource1");
            QVERIFY(middle);
            middle->click();
        }
    }));
    QCOMPARE(frame_->documentCount(), 2);
    QCOMPARE(frame_->activeView()->layer_count(), 1);
    QCOMPARE(frame_->activeView()->document_depth_bits(), 32);
    QVERIFY(frame_->activeView()->composite_argb(1, 1) != full);
}

void MergeToHdrTest::editsToneCurve()
{
    QVERIFY(run());
    const auto baseline = frame_->activeView()->composite_argb(1, 1);
    bool previewSeen = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage != Stage::Preview) { return; }
        previewSeen = true;
        auto* tabs = control<QTabWidget>(dialog, "hdrMergeTabs");
        auto* image = control<QLabel>(dialog, "hdrMergeImage");
        QVERIFY(tabs && image);
        QCOMPARE(tabs->tabText(0), QStringLiteral("Advanced"));
        QCOMPARE(tabs->tabText(1), QStringLiteral("Curve"));
        QVERIFY(tabs->widget(0)->findChild<QSpinBox*>(QStringLiteral("hdrShadow")));
        tabs->setCurrentIndex(1);
        auto* curve = control<pictura::CurveWidget>(dialog, "hdrMergeToneCurve");
        auto* input = control<QDoubleSpinBox>(dialog, "hdrMergeCurveInput");
        auto* output = control<QDoubleSpinBox>(dialog, "hdrMergeCurveOutput");
        auto* corner = control<QCheckBox>(dialog, "hdrMergeCurveCorner");
        auto* reset = control<QPushButton>(dialog, "hdrMergeCurveReset");
        QVERIFY(curve && input && output && corner && reset);
        QCOMPARE(curve->points().size(), 2);
        QCOMPARE(curve->points().first(), QPointF(0, 0));
        QCOMPARE(curve->points().last(), QPointF(1, 1));
        QVERIFY(!input->isEnabled() && !output->isEnabled());
        const QImage identity = image->pixmap().toImage();
        QTest::mousePress(curve, Qt::LeftButton, Qt::NoModifier, QPoint(65, 45));
        QCOMPARE(curve->points().size(), 3);
        const QPointF inserted = curve->points().at(1);
        QTest::mouseMove(curve, QPoint(80, 55));
        QTest::mouseRelease(curve, Qt::LeftButton, Qt::NoModifier, QPoint(80, 55));
        QVERIFY(curve->points().at(1) != inserted);
        QVERIFY(input->isEnabled() && output->isEnabled());
        input->setValue(40);
        output->setValue(15);
        QVERIFY(QMetaObject::invokeMethod(output, "editingFinished"));
        QVERIFY(qAbs(curve->points().at(1).x() - 0.4) < 0.005);
        QVERIFY(qAbs(curve->points().at(1).y() - 0.15) < 0.005);
        corner->click();
        QVERIFY(corner->isChecked());
        QVERIFY(identity != image->pixmap().toImage());
        reset->click();
        QVERIFY(!corner->isChecked());
        QCOMPARE(curve->points().size(), 2);
        QCOMPARE(image->pixmap().toImage(), identity);
        QTest::mouseClick(curve, Qt::LeftButton, Qt::NoModifier, QPoint(65, 45));
        output->setValue(10);
        QVERIFY(QMetaObject::invokeMethod(output, "editingFinished"));
    }));
    QVERIFY(previewSeen);
    QCOMPARE(frame_->activeView()->document_depth_bits(), 32);
    QVERIFY(frame_->activeView()->composite_argb(1, 1) != baseline);
}

void MergeToHdrTest::loadsLegacyTonePreset()
{
    const QString path = fixtures_.filePath(QStringLiteral("legacy.phdr-preset"));
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly));
    QVERIFY(file.write(R"({"kind":"pictura-hdr-preset","version":1,"align":false,"ghosts":false,"depth":32,"previewEv":0,"tone":[187,4,0.99,0,30,0,0,0,20,0,0.5,0]})") > 0);
    file.close();
    pictura::setOpenFileNameProviderForTest([&](QWidget*, const QString&, const QString&,
                                                const QStringList&, QString*) { return path; });
    const auto resetProvider = qScopeGuard([] { pictura::setOpenFileNameProviderForTest({}); });
    bool previewSeen = false;
    QVERIFY(run([&](Stage stage, QDialog& dialog) {
        if (stage != Stage::Preview) { return; }
        previewSeen = true;
        auto* presets = control<QComboBox>(dialog, "hdrMergePreset");
        auto* curve = control<pictura::CurveWidget>(dialog, "hdrMergeToneCurve");
        auto* reset = control<QPushButton>(dialog, "hdrMergeCurveReset");
        auto* image = control<QLabel>(dialog, "hdrMergeImage");
        auto* zoom = control<QComboBox>(dialog, "hdrMergeZoom");
        QVERIFY(presets && curve && reset && image && zoom);
        const int actualSize = zoom->findData(100);
        QVERIFY(actualSize >= 0);
        // Full-screen layout can resize the Fit preview while the file picker is open.
        zoom->setCurrentIndex(actualSize);
        const QImage baseline = image->pixmap().toImage();
        QVERIFY(QMetaObject::invokeMethod(presets, "activated", Q_ARG(int, 1)));
        QCOMPARE(curve->points().size(), 2);
        QVERIFY(image->pixmap().toImage() != baseline);
        reset->click();
        QCOMPARE(image->pixmap().toImage(), baseline);
        dialog.reject();
    }));
    QVERIFY(previewSeen);
    QCOMPARE(frame_->documentCount(), 0);
}

void MergeToHdrTest::budgetFailureIsAtomic()
{
    pictura::set_available_memory_for_test(0);
    const auto restore = qScopeGuard([] { pictura::set_available_memory_for_test(-1); });
    bool reported = false;
    QVERIFY(run([&](Stage stage, QDialog&) { reported |= stage == Stage::Warning; }));
    QVERIFY(reported);
    QCOMPARE(frame_->documentCount(), 0);
}

PICTURA_QT_TEST_MAIN(MergeToHdrTest)
#include "tst_merge_to_hdr.moc"
