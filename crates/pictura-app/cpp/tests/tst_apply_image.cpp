// Image > Apply Image (#609): the dialog's source and mode lists, preview,
// cancel and commit, a preview that reads its own document, selection and mask
// confinement, a layer-mask target, and a native-depth cancel.

#include <QtTest/QtTest>

#include "apply_image_dialog.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"

#include "api/document_bridge.h"
#include "api/layers_bridge.h"

#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QSpinBox>

#include "qt_test_support.h"

using pictura::ApplyImageDialog;

class ApplyImageTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void menuRowIsLive();
    void sourcesMatchDimensions();
    void compositeHiddenAcrossModes();
    void scaleOffsetFollowMode();
    void modeList();
    void selectionAsMask();
    void selectionConfines();
    void cancelRestores();
    void okCommitsOneState();
    void mergedDoesNotCompound();
    void maskTarget();
    void cancelRestoresNativeStore();

private:
    pictura::PictureView* open(const QColor& color, const QString& name, QSize size = {20, 20},
                               QImage::Format format = QImage::Format_RGB32);
    QList<ApplyImageDialog::Source> documents() const;
    static void pick(ApplyImageDialog& dialog, const char* combo, const QString& text);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void ApplyImageTest::initTestCase()
{
    QVERIFY(stateHome_.isValid() && dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ApplyImageTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

pictura::PictureView* ApplyImageTest::open(const QColor& color, const QString& name, QSize size,
                                           QImage::Format format)
{
    QImage image(size, format);
    image.fill(color);
    const QString path = dir_.filePath(name);
    if (!image.save(path) || !window_->openDocumentAtPath(path)) {
        return nullptr;
    }
    QCoreApplication::processEvents();
    return window_->activeView();
}

QList<ApplyImageDialog::Source> ApplyImageTest::documents() const
{
    QList<ApplyImageDialog::Source> out;
    for (int i = 0; i < window_->documentCount(); ++i) {
        out.append({window_->documentName(i), window_->viewAt(i)});
    }
    return out;
}

void ApplyImageTest::pick(ApplyImageDialog& dialog, const char* combo, const QString& text)
{
    auto* box = dialog.findChild<QComboBox*>(QString::fromUtf8(combo));
    QVERIFY(box);
    const int index = box->findText(text);
    QVERIFY2(index >= 0, qPrintable(text));
    box->setCurrentIndex(index);
}

void ApplyImageTest::menuRowIsLive()
{
    QAction* action = window_->registry()->action(pictura::commandIdForPath(
        {QStringLiteral("Image"), QStringLiteral("Apply Image…")}));
    QVERIFY(action);
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(open(Qt::red, QStringLiteral("menu.png")));
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
}

void ApplyImageTest::sourcesMatchDimensions()
{
    QVERIFY(open(Qt::red, QStringLiteral("same.png")));
    QVERIFY(open(Qt::blue, QStringLiteral("other.png"), {30, 30}));
    pictura::PictureView* active = open(Qt::green, QStringLiteral("active.png"));
    QVERIFY(active);
    ApplyImageDialog dialog(active, window_->imageView(), documents());
    auto* source = dialog.findChild<QComboBox*>(QStringLiteral("applyImageSource"));
    QVERIFY(source);
    QCOMPARE(source->count(), 2);
    QCOMPARE(source->itemText(0), QStringLiteral("active.png"));
    QCOMPARE(source->itemText(1), QStringLiteral("same.png"));
    dialog.reject();
}

void ApplyImageTest::compositeHiddenAcrossModes()
{
    pictura::PictureView gray;
    QVERIFY(pictura::create_document(gray, 20, 20, QStringLiteral("grayscale"), 8, 0x808080u, false,
                                     72.0, false));
    pictura::PictureView* active = open(Qt::red, QStringLiteral("rgb.png"));
    QVERIFY(active);
    QList<ApplyImageDialog::Source> sources = documents();
    sources.append({QStringLiteral("gray"), &gray});
    ApplyImageDialog dialog(active, window_->imageView(), sources);
    auto* source = dialog.findChild<QComboBox*>(QStringLiteral("applyImageSource"));
    auto* channel = dialog.findChild<QComboBox*>(QStringLiteral("applyImageChannel"));
    QVERIFY(source && channel);
    QVERIFY(channel->findData(0) >= 0);
    source->setCurrentIndex(source->findText(QStringLiteral("gray")));
    QCOMPARE(channel->findData(0), -1);
    QVERIFY(channel->findText(QStringLiteral("Gray")) >= 0);
    dialog.reject();
}

void ApplyImageTest::scaleOffsetFollowMode()
{
    pictura::PictureView* view = open(Qt::red, QStringLiteral("mode.png"));
    QVERIFY(view);
    ApplyImageDialog dialog(view, window_->imageView(), documents());
    dialog.show();
    auto* scale = dialog.findChild<QDoubleSpinBox*>(QStringLiteral("applyImageScale"));
    auto* offset = dialog.findChild<QSpinBox*>(QStringLiteral("applyImageOffset"));
    QVERIFY(scale && offset);
    QVERIFY(!scale->isVisible() && !offset->isVisible());
    for (const QString& mode : {QStringLiteral("Add"), QStringLiteral("Subtract")}) {
        pick(dialog, "applyImageBlending", mode);
        QVERIFY(scale->isVisible() && offset->isVisible());
    }
    pick(dialog, "applyImageBlending", QStringLiteral("Screen"));
    QVERIFY(!scale->isVisible() && !offset->isVisible());
    QCOMPARE(scale->minimum(), 1.0);
    QCOMPARE(scale->maximum(), 2.0);
    QCOMPARE(offset->minimum(), -255);
    QCOMPARE(offset->maximum(), 255);
    dialog.reject();
}

void ApplyImageTest::modeList()
{
    pictura::PictureView* view = open(Qt::red, QStringLiteral("list.png"));
    QVERIFY(view);
    ApplyImageDialog dialog(view, window_->imageView(), documents());
    auto* blending = dialog.findChild<QComboBox*>(QStringLiteral("applyImageBlending"));
    QVERIFY(blending);
    QCOMPARE(blending->currentText(), QStringLiteral("Multiply"));
    QCOMPARE(blending->findText(QStringLiteral("Dissolve")), -1);
    int subtracts = 0;
    for (int i = 0; i < blending->count(); ++i) {
        subtracts += blending->itemText(i) == QLatin1String("Subtract");
    }
    QCOMPARE(subtracts, 1);
    QVERIFY(blending->findText(QStringLiteral("Add")) >= 0);
    dialog.reject();
}

void ApplyImageTest::selectionAsMask()
{
    pictura::PictureView* source = open(Qt::white, QStringLiteral("source.png"));
    QVERIFY(source);
    QVERIFY(source->select_rect(0, 0, 10, 20, QStringLiteral("new"), 0.0));
    pictura::PictureView* target = open(QColor(40, 40, 40), QStringLiteral("target.png"));
    QVERIFY(target);
    ApplyImageDialog dialog(target, window_->imageView(), documents());
    pick(dialog, "applyImageSource", QStringLiteral("source.png"));
    pick(dialog, "applyImageBlending", QStringLiteral("Normal"));
    dialog.findChild<QCheckBox*>(QStringLiteral("applyImageUseMask"))->setChecked(true);
    pick(dialog, "applyImageMaskSource", QStringLiteral("source.png"));
    pick(dialog, "applyImageMaskChannel", QStringLiteral("Selection"));
    dialog.accept();
    QCOMPARE(qRed(target->composite_argb(5, 5)), 255);
    QCOMPARE(qRed(target->composite_argb(15, 5)), 40);
}

void ApplyImageTest::selectionConfines()
{
    QVERIFY(open(Qt::white, QStringLiteral("source.png")));
    pictura::PictureView* target = open(QColor(40, 40, 40), QStringLiteral("target.png"));
    QVERIFY(target);
    QVERIFY(target->select_rect(0, 0, 10, 20, QStringLiteral("new"), 0.0));
    ApplyImageDialog dialog(target, window_->imageView(), documents());
    pick(dialog, "applyImageSource", QStringLiteral("source.png"));
    pick(dialog, "applyImageBlending", QStringLiteral("Normal"));
    dialog.accept();
    QCOMPARE(qRed(target->composite_argb(5, 5)), 255);
    QCOMPARE(qRed(target->composite_argb(15, 5)), 40);
}

void ApplyImageTest::cancelRestores()
{
    pictura::PictureView* view = open(QColor(100, 150, 200), QStringLiteral("cancel.png"));
    QVERIFY(view);
    const QRgb before = view->composite_argb(5, 5);
    const int history = view->history_count();
    {
        ApplyImageDialog dialog(view, window_->imageView(), documents());
        QVERIFY(view->composite_argb(5, 5) != before);
        dialog.reject();
    }
    QCOMPARE(view->composite_argb(5, 5), before);
    QCOMPARE(view->history_count(), history);
}

void ApplyImageTest::okCommitsOneState()
{
    pictura::PictureView* view = open(QColor(100, 150, 200), QStringLiteral("ok.png"));
    QVERIFY(view);
    const QRgb before = view->composite_argb(5, 5);
    const int history = view->history_count();
    ApplyImageDialog dialog(view, window_->imageView(), documents());
    dialog.accept();
    QVERIFY(view->composite_argb(5, 5) != before);
    QCOMPARE(view->history_count(), history + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Apply Image"));
    QVERIFY(view->undo());
    QCOMPARE(view->composite_argb(5, 5), before);
}

void ApplyImageTest::mergedDoesNotCompound()
{
    pictura::PictureView* view = open(QColor(100, 100, 100), QStringLiteral("self.png"));
    QVERIFY(view);
    ApplyImageDialog dialog(view, window_->imageView(), documents());
    auto* opacity = dialog.findChild<QSpinBox*>(QStringLiteral("applyImageOpacity"));
    QVERIFY(opacity);
    opacity->setValue(99);
    opacity->setValue(100);
    QVERIFY(qAbs(qRed(view->composite_argb(5, 5)) - 39) <= 1);
    pick(dialog, "applyImageChannel", QStringLiteral("Red"));
    QVERIFY(qAbs(qRed(view->composite_argb(5, 5)) - 39) <= 1);
    dialog.accept();
    QVERIFY(qAbs(qRed(view->composite_argb(5, 5)) - 39) <= 1);
}

void ApplyImageTest::maskTarget()
{
    QVERIFY(open(Qt::black, QStringLiteral("black.png")));
    pictura::PictureView* view = open(QColor(100, 150, 200), QStringLiteral("masked.png"), {20, 20},
                                      QImage::Format_ARGB32);
    QVERIFY(view);
    const QString layer = QStringLiteral("0");
    QVERIFY(pictura::layer_mask_add_path(*view, layer, QStringLiteral("reveal-all")));
    QVERIFY(pictura::mask_edit_target_set(*view, layer));
    const QRgb before = view->composite_argb(5, 5);
    ApplyImageDialog dialog(view, window_->imageView(), documents());
    pick(dialog, "applyImageSource", QStringLiteral("black.png"));
    pick(dialog, "applyImageBlending", QStringLiteral("Normal"));
    dialog.accept();
    QVERIFY(view->composite_argb(5, 5) != before);
    QVERIFY(pictura::layer_mask_set_enabled_path(*view, layer, false));
    QCOMPARE(view->composite_argb(5, 5), before);
}

void ApplyImageTest::cancelRestoresNativeStore()
{
    pictura::PictureView view;
    QVERIFY(pictura::create_document(view, 16, 16, QStringLiteral("rgb"), 16, 0x806040u, false,
                                     72.0, false));
    QCOMPARE(view.document_depth_bits(), 16);
    const QRgb before = view.composite_argb(5, 5);
    {
        ApplyImageDialog dialog(&view, nullptr, {{QStringLiteral("deep"), &view}});
        QVERIFY(view.composite_argb(5, 5) != before);
        dialog.reject();
    }
    QCOMPARE(view.composite_argb(5, 5), before);
    QCOMPARE(view.document_depth_bits(), 16);
}

#include "tst_apply_image.moc"

QTEST_MAIN(ApplyImageTest)
