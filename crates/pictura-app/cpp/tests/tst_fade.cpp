#include <QtTest/QtTest>

#include <functional>

#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>

#include "commands.h"
#include "fade_dialog.h"
#include "frame.h"
#include "image_view.h"

#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "api/layers_bridge.h"
#include "api/paint_bridge.h"
#include "api/selection_bridge.h"

#include "qt_test_support.h"

namespace {

pictura::FadeDialog* activeFadeDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::FadeDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

QAction* fadeAction(pictura::PicturaMainWindow* window)
{
    return window->registry()->action(QString::fromLatin1(pictura::command_ids::EditFade));
}

void applyFilter(pictura::PictureView* view)
{
    // Clouds changes a uniform white layer; a blur of it would be a no-op.
    QVERIFY(pictura::apply_filter_params(*view, QStringLiteral("clouds"), {}));
}

} // namespace

class FadeTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void fadeStartsDisabled();
    void filterCommitEnablesFade();
    void undoDisablesFade();
    void anotherEditDisablesFade();
    void adjustmentEnablesFade();
    void paintEnablesFade();
    void maskAndSmartFilterDoNotEnableFade();
    void dialogOpensOnRetainedValues();
    void previewUpdatesCanvasWithoutHistory();
    void cancelRestoresPixels();
    void okCommitsOneState();
    void reopeningRecomputesFromEffectOutput();
    void labDisablesRestrictedModes();
    void commandIsImplementedAndDispatchable();

private:
    pictura::PictureView* newRgbDocument(const QString& name);
    bool withDialog(const std::function<void(pictura::FadeDialog*)>& run);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void FadeTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

pictura::PictureView* FadeTest::newRgbDocument(const QString& name)
{
    if (!window_->newDocument(name, 32, 32, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return nullptr;
    }
    window_->registry()->refresh();
    return window_->activeView();
}

bool FadeTest::withDialog(const std::function<void(pictura::FadeDialog*)>& run)
{
    bool seen = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = activeFadeDialog();
        if (!dialog) {
            return;
        }
        seen = true;
        run(dialog);
    });
    window_->registry()->dispatch(QString::fromLatin1(pictura::command_ids::EditFade));
    return seen;
}

void FadeTest::fadeStartsDisabled()
{
    QVERIFY(newRgbDocument(QStringLiteral("FadeDisabled")) != nullptr);
    QAction* fade = fadeAction(window_.get());
    QVERIFY(fade != nullptr);
    QVERIFY(!fade->isEnabled());
    QCOMPARE(fade->text(), QStringLiteral("Fade…"));
}

void FadeTest::filterCommitEnablesFade()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeFilter"));
    QVERIFY(view != nullptr);
    applyFilter(view);
    window_->registry()->refresh();
    QVERIFY(pictura::fade_available(*view));
    QAction* fade = fadeAction(window_.get());
    QVERIFY(fade->isEnabled());
    QCOMPARE(fade->text(), QStringLiteral("Fade Clouds…"));
}

void FadeTest::undoDisablesFade()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeUndo"));
    QVERIFY(view != nullptr);
    applyFilter(view);
    QVERIFY(pictura::fade_available(*view));
    QVERIFY(view->undo());
    window_->registry()->refresh();
    QVERIFY(!pictura::fade_available(*view));
    QVERIFY(!fadeAction(window_.get())->isEnabled());
}

void FadeTest::anotherEditDisablesFade()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeOtherEdit"));
    QVERIFY(view != nullptr);
    applyFilter(view);
    QVERIFY(pictura::fade_available(*view));
    QVERIFY(pictura::edit_fill(*view, 0xff000000u, -1, QStringLiteral("normal"), 100, false));
    window_->registry()->refresh();
    QVERIFY(!pictura::fade_available(*view));
    QVERIFY(!fadeAction(window_.get())->isEnabled());
}

void FadeTest::adjustmentEnablesFade()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeAdjust"));
    QVERIFY(view != nullptr);
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
    window_->registry()->refresh();
    QVERIFY(pictura::fade_available(*view));
    QCOMPARE(fadeAction(window_.get())->text(), QStringLiteral("Fade Invert…"));
}

void FadeTest::paintEnablesFade()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadePaint"));
    QVERIFY(view != nullptr);
    QVERIFY(view->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 8, 100, 100, 0, 100, 100, 25,
                              QStringLiteral("normal"), false, false));
    QVERIFY(view->paint_dab(16.0, 16.0, 1.0));
    QVERIFY(view->end_paint());
    window_->registry()->refresh();
    QVERIFY(pictura::fade_available(*view));
    QCOMPARE(fadeAction(window_.get())->text(), QStringLiteral("Fade Brush…"));
}

void FadeTest::maskAndSmartFilterDoNotEnableFade()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeMask"));
    QVERIFY(view != nullptr);
    QVERIFY(pictura::quick_mask_set(*view, true));
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
    QVERIFY(!pictura::fade_available(*view));
    QVERIFY(pictura::quick_mask_set(*view, false));

    pictura::PictureView* smart = newRgbDocument(QStringLiteral("FadeSmart"));
    QVERIFY(smart != nullptr);
    const QString path = smart->add_solid_fill(0xff808080u);
    QVERIFY(!path.isEmpty());
    QVERIFY(smart->rasterize_fill_content(path));
    QVERIFY(smart->convert_to_smart_object(path));
    QVERIFY(pictura::apply_filter_params(*smart, QStringLiteral("gaussian-blur"), {1.0}));
    QVERIFY(!pictura::fade_available(*smart));
}

void FadeTest::dialogOpensOnRetainedValues()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadePrefill"));
    QVERIFY(view != nullptr);
    applyFilter(view);
    QVERIFY(pictura::fade_apply(*view, 40, QStringLiteral("mul ")));
    bool checked = false;
    QVERIFY(withDialog([&](pictura::FadeDialog* dialog) {
        auto* opacity = dialog->findChild<QSpinBox*>(QStringLiteral("fadeOpacity"));
        auto* mode = dialog->findChild<QComboBox*>(QStringLiteral("fadeMode"));
        auto* preview = dialog->findChild<QCheckBox*>(QStringLiteral("fadePreview"));
        checked = opacity && opacity->value() == 40 && mode && mode->currentData().toString()
                      == QStringLiteral("mul ")
                  && preview && preview->isChecked();
        dialog->reject();
    }));
    QVERIFY(checked);
}

void FadeTest::previewUpdatesCanvasWithoutHistory()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadePreview"));
    QVERIFY(view != nullptr);
    applyFilter(view);
    const QImage before = view->image();
    const int states = view->history_count();
    bool changed = false;
    bool historyStable = false;
    QVERIFY(withDialog([&](pictura::FadeDialog* dialog) {
        auto* opacity = dialog->findChild<QSpinBox*>(QStringLiteral("fadeOpacity"));
        if (!opacity) {
            dialog->reject();
            return;
        }
        opacity->setValue(50);
        changed = view->image() != before;
        historyStable = view->history_count() == states;
        dialog->reject();
    }));
    QVERIFY(changed);
    QVERIFY(historyStable);
    QVERIFY(view->image() == before);
    QCOMPARE(view->history_count(), states);
}

void FadeTest::cancelRestoresPixels()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeCancel"));
    QVERIFY(view != nullptr);
    applyFilter(view);
    const QImage before = view->image();
    const int states = view->history_count();
    QVERIFY(withDialog([&](pictura::FadeDialog* dialog) {
        auto* opacity = dialog->findChild<QSpinBox*>(QStringLiteral("fadeOpacity"));
        if (opacity) {
            opacity->setValue(20);
        }
        dialog->reject();
    }));
    QVERIFY(view->image() == before);
    QCOMPARE(view->history_count(), states);
}

void FadeTest::okCommitsOneState()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeCommit"));
    QVERIFY(view != nullptr);
    applyFilter(view);
    const int states = view->history_count();
    QVERIFY(withDialog([&](pictura::FadeDialog* dialog) {
        auto* opacity = dialog->findChild<QSpinBox*>(QStringLiteral("fadeOpacity"));
        if (opacity) {
            opacity->setValue(50);
        }
        dialog->accept();
    }));
    QCOMPARE(view->history_count(), states + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Fade Clouds"));
    QCOMPARE(pictura::fade_opacity(*view), 50);
}

void FadeTest::reopeningRecomputesFromEffectOutput()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeReopen"));
    QVERIFY(view != nullptr);
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
    QVERIFY(pictura::fade_apply(*view, 50, QStringLiteral("norm")));
    QVERIFY(pictura::fade_apply(*view, 25, QStringLiteral("norm")));
    // White inverted to black, faded at 25 % back toward white is 191. A
    // compounding fade over the 50 % result would be 96.
    QCOMPARE(view->image().pixelColor(0, 0).red(), 191);
}

void FadeTest::labDisablesRestrictedModes()
{
    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeLab"));
    QVERIFY(view != nullptr);
    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("lab")));
    QVERIFY(pictura::image_adjust_direct(*view, QStringLiteral("invert")));
    bool checked = false;
    QVERIFY(withDialog([&](pictura::FadeDialog* dialog) {
        auto* mode = dialog->findChild<QComboBox*>(QStringLiteral("fadeMode"));
        auto* model = mode ? qobject_cast<QStandardItemModel*>(mode->model()) : nullptr;
        if (!mode || !model) {
            dialog->reject();
            return;
        }
        for (const QString& key :
             {QStringLiteral("idiv"), QStringLiteral("div "), QStringLiteral("lite"),
              QStringLiteral("dark"), QStringLiteral("diff"), QStringLiteral("smud")}) {
            const int index = mode->findData(key);
            if (index < 0 || !model->item(index) || model->item(index)->isEnabled()) {
                dialog->reject();
                return;
            }
        }
        checked = true;
        dialog->reject();
    }));
    QVERIFY(checked);
}

void FadeTest::commandIsImplementedAndDispatchable()
{
    bool implemented = false;
    for (const pictura::CommandInfo& info : window_->registry()->describe()) {
        if (info.id == QLatin1String(pictura::command_ids::EditFade)) {
            implemented = info.implemented;
        }
    }
    QVERIFY(implemented);

    pictura::PictureView* view = newRgbDocument(QStringLiteral("FadeDispatch"));
    QVERIFY(view != nullptr);
    applyFilter(view);
    bool opened = false;
    QVERIFY(withDialog([&](pictura::FadeDialog* dialog) {
        opened = true;
        dialog->reject();
    }));
    QVERIFY(opened);
}

QTEST_MAIN(FadeTest)

#include "tst_fade.moc"
