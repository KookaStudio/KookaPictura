#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QRadioButton>

#include "commands.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "image_view.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

const QStringList kDeInterlacePath = {QStringLiteral("Filter"), QStringLiteral("Video"),
                                      QStringLiteral("De-Interlace")};

pictura::FilterPreviewDialog* activeFilterDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::FilterPreviewDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

} // namespace

class DeInterlaceTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void dialogFollowsCs6();
    void oddFieldsInterpolateAndCommitOneState();
    void noFilterTargetDisablesTheEntry();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QString id_;
};

void DeInterlaceTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);

    // Every row is a different colour, so an interpolated row cannot equal the
    // row it replaces by coincidence.
    QImage pattern(32, 32, QImage::Format_RGB32);
    for (int y = 0; y < 32; ++y) {
        for (int x = 0; x < 32; ++x) {
            pattern.setPixelColor(x, y,
                                  QColor((y * y * 11 + 20) % 200, (y * 13 + 40) % 200,
                                         (y * 5 + 60) % 200));
        }
    }
    const QString path = dir_.filePath(QStringLiteral("interlaced.png"));
    QVERIFY(pattern.save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);

    const pictura::FilterCommandSpec* spec = pictura::filterCommandForPath(kDeInterlacePath);
    QVERIFY(spec != nullptr);
    id_ = pictura::commandIdForPath(kDeInterlacePath);
}

// Two radio groups, both opening on the first choice, with a live preview.
void DeInterlaceTest::dialogFollowsCs6()
{
    const pictura::FilterCommandSpec& spec = *pictura::filterCommandForPath(kDeInterlacePath);
    QVERIFY(spec.livePreview);
    QCOMPARE(spec.params.size(), 2);
    pictura::FilterPreviewDialog dialog(window_->activeView(), spec);
    QCOMPARE(dialog.windowTitle(), QStringLiteral("De-Interlace"));
    QCOMPARE(dialog.values(), QList<double>({0.0, 0.0}));

    QStringList labels;
    QStringList checked;
    for (QRadioButton* radio : dialog.findChildren<QRadioButton*>()) {
        labels.append(radio->text());
        if (radio->isChecked()) {
            checked.append(radio->text());
        }
    }
    QCOMPARE(labels, QStringList({QStringLiteral("Odd Fields"), QStringLiteral("Even Fields"),
                                  QStringLiteral("Interpolation"),
                                  QStringLiteral("Duplication")}));
    QCOMPARE(checked, QStringList({QStringLiteral("Odd Fields"),
                                   QStringLiteral("Interpolation")}));
    dialog.reject();
}

void DeInterlaceTest::oddFieldsInterpolateAndCommitOneState()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QImage before = view->image();

    QList<double> seen;
    QTimer::singleShot(0, [&seen] {
        if (auto* dialog = activeFilterDialog()) {
            seen = dialog->values();
            dialog->accept();
        }
    });
    const int states = view->history_count();
    QVERIFY(window_->registry()->dispatch(id_));
    QCOMPARE(seen, QList<double>({0.0, 0.0}));
    QCOMPARE(view->history_count(), states + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Filter: De-Interlace"));

    const QImage after = view->image();
    QVERIFY(after != before);
    // Even rows are retained; an odd row is the mean of the rows around it.
    QCOMPARE(after.pixelColor(5, 0), before.pixelColor(5, 0));
    QCOMPARE(after.pixelColor(5, 2), before.pixelColor(5, 2));
    const QColor top = before.pixelColor(5, 0);
    const QColor bottom = before.pixelColor(5, 2);
    const QColor middle = after.pixelColor(5, 1);
    QVERIFY(qAbs(middle.red() - (top.red() + bottom.red()) / 2) <= 4);
    QVERIFY(qAbs(middle.green() - (top.green() + bottom.green()) / 2) <= 4);
    QVERIFY(qAbs(middle.blue() - (top.blue() + bottom.blue()) / 2) <= 4);
}

void DeInterlaceTest::noFilterTargetDisablesTheEntry()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    registry->refresh();

    QAction* action = registry->action(id_);
    QVERIFY(action != nullptr);
    QVERIFY(action->isEnabled());
    QVERIFY(action->text().endsWith(QStringLiteral("…")));

    const QString previous = view->active_layer_path();
    view->set_active_layer(QString());
    registry->refresh();
    action = registry->action(id_);
    QVERIFY(action != nullptr);
    QVERIFY(!action->isEnabled());

    view->set_active_layer(previous);
    registry->refresh();
    action = registry->action(id_);
    QVERIFY(action != nullptr);
    QVERIFY(action->isEnabled());
}

QTEST_MAIN(DeInterlaceTest)

#include "tst_de_interlace.moc"
