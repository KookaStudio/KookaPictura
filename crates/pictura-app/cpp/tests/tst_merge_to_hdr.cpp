#include <QtTest/QtTest>

#include "api/paint_bridge.h"
#include "api/view_bridge.h"
#include "commands.h"
#include "dialogs.h"
#include "frame.h"
#include "hdr_conversion_dialog.h"
#include "load_layers_dialog.h"
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
