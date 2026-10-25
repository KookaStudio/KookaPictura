// Open-time profile warnings: the ask preferences, the three choices, and the
// abort path.

#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>

#include "color_settings_dialog.h"
#include "frame.h"
#include "profile_warning_dialog.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/color_management.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

namespace {

template <typename T>
T* findTopLevel(const char* name)
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<T*>(widget); dialog && dialog->objectName() == name) {
            return dialog;
        }
    }
    return nullptr;
}

} // namespace

class ProfileWarningsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void psdMismatchPreserve();
    void psdMismatchConvert();
    void psdMismatchDiscard();
    void missingAssignWorking();
    void missingAssignConvert();
    void missingLeaveUntagged();
    void cancelAborts();
    void cmykMismatch();
    void rasterMismatchPreserve();
    void disabled();
    void defaultsOff();
    void preferenceSurvivesRestart();
    void notADocumentEdit();

private:
    struct Fixture {
        QString path;
        QRgb sample = 0;
    };

    Fixture makePsd(const QString& name, bool adobe, bool cmyk);
    QString makeJpeg(const QString& name, bool adobe);
    void setAsks(bool mismatch, bool missing);
    void answerNext(bool accept, pictura::ProfileWarningDialog::Choice choice);
    void stopAnswering() { answerTimer_.stop(); }
    bool openWith(const QString& path, bool accept,
                  pictura::ProfileWarningDialog::Choice choice);
    QString profileName() const { return pictura::document_profile_name(*window_->activeView()); }
    void closeActive() { window_->closeDocument(window_->activeDocumentIndex(), false); }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
    QTimer answerTimer_;
    bool answerAccept_ = true;
    pictura::ProfileWarningDialog::Choice answerChoice_ =
        pictura::ProfileWarningDialog::UseEmbedded;
};

void ProfileWarningsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_);
    answerTimer_.setInterval(10);
    connect(&answerTimer_, &QTimer::timeout, this, [this]() {
        auto* dialog = findTopLevel<pictura::ProfileWarningDialog>("profileWarningDialog");
        if (!dialog) {
            return;
        }
        answerTimer_.stop();
        if (answerAccept_) {
            dialog->setChoiceForTest(answerChoice_);
            dialog->accept();
        } else {
            dialog->reject();
        }
    });
}

ProfileWarningsTest::Fixture ProfileWarningsTest::makePsd(
    const QString& name, bool adobe, bool cmyk)
{
    Fixture fixture;
    if (!window_->newDocument(QStringLiteral("Warn"), 8, 8, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return fixture;
    }
    pictura::PictureView* view = window_->activeView();
    view->select_all();
    pictura::edit_fill(*view, QColor(30, 120, 220).rgba(), -1, QStringLiteral("normal"), 100,
                       false);
    view->deselect();
    if (adobe && !view->assign_profile(1)) {
        closeActive();
        return fixture;
    }
    if (cmyk && !pictura::convert_image_mode(*view, QStringLiteral("cmyk"))) {
        closeActive();
        return fixture;
    }
    fixture.sample = view->sample_argb(2, 2);
    fixture.path = dir_.filePath(name);
    if (!view->save(fixture.path)) {
        fixture.path.clear();
    }
    closeActive();
    return fixture;
}

QString ProfileWarningsTest::makeJpeg(const QString& name, bool adobe)
{
    if (!window_->newDocument(QStringLiteral("Warn"), 8, 8, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return {};
    }
    pictura::PictureView* view = window_->activeView();
    view->select_all();
    pictura::edit_fill(*view, QColor(30, 120, 220).rgba(), -1, QStringLiteral("normal"), 100,
                       false);
    view->deselect();
    if (adobe && !view->assign_profile(1)) {
        closeActive();
        return {};
    }
    pictura::set_save_embed_profile(*view, true);
    const QString path = dir_.filePath(name);
    const bool saved = view->save(path);
    closeActive();
    return saved ? path : QString();
}

void ProfileWarningsTest::setAsks(bool mismatch, bool missing)
{
    auto* timer = new QTimer(this);
    timer->setInterval(10);
    connect(timer, &QTimer::timeout, this, [timer, mismatch, missing]() {
        auto* dialog = findTopLevel<pictura::ColorSettingsDialog>("colorSettingsDialog");
        if (!dialog) {
            return;
        }
        timer->stop();
        if (auto* box = dialog->findChild<QCheckBox*>(QStringLiteral("colorSettingsAskMismatch"))) {
            box->setChecked(mismatch);
        }
        if (auto* box = dialog->findChild<QCheckBox*>(QStringLiteral("colorSettingsAskMissing"))) {
            box->setChecked(missing);
        }
        dialog->accept();
        timer->deleteLater();
    });
    timer->start();
    QTimer::singleShot(5000, timer, [timer]() {
        timer->stop();
        timer->deleteLater();
    });
    window_->showColorSettings();
    timer->stop();
    timer->deleteLater();
}

void ProfileWarningsTest::answerNext(bool accept, pictura::ProfileWarningDialog::Choice choice)
{
    answerAccept_ = accept;
    answerChoice_ = choice;
    answerTimer_.start();
}

bool ProfileWarningsTest::openWith(const QString& path, bool accept,
                                   pictura::ProfileWarningDialog::Choice choice)
{
    answerNext(accept, choice);
    const bool opened = window_->openPath(path);
    stopAnswering();
    return opened;
}

void ProfileWarningsTest::psdMismatchPreserve()
{
    const Fixture fixture = makePsd(QStringLiteral("adobe.psd"), true, false);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(true, false);
    QVERIFY(openWith(fixture.path, true, pictura::ProfileWarningDialog::UseEmbedded));
    QVERIFY2(profileName() == QStringLiteral("RGB built-in"), qPrintable(profileName()));
    QCOMPARE(window_->activeView()->sample_argb(2, 2), fixture.sample);
    closeActive();
}

void ProfileWarningsTest::psdMismatchConvert()
{
    const Fixture fixture = makePsd(QStringLiteral("adobe-convert.psd"), true, false);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(true, false);
    QVERIFY(openWith(fixture.path, true, pictura::ProfileWarningDialog::ConvertToWorking));
    QVERIFY(profileName().contains(QStringLiteral("sRGB")));
    QVERIFY(window_->activeView()->sample_argb(2, 2) != fixture.sample);
    closeActive();
}

void ProfileWarningsTest::psdMismatchDiscard()
{
    const Fixture fixture = makePsd(QStringLiteral("adobe-discard.psd"), true, false);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(true, false);
    QVERIFY(openWith(fixture.path, true, pictura::ProfileWarningDialog::DiscardEmbedded));
    QVERIFY(profileName().contains(QStringLiteral("sRGB")));
    QCOMPARE(window_->activeView()->sample_argb(2, 2), fixture.sample);
    closeActive();
}

void ProfileWarningsTest::missingAssignWorking()
{
    const Fixture fixture = makePsd(QStringLiteral("untagged.psd"), false, false);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(false, true);
    QVERIFY(openWith(fixture.path, true, pictura::ProfileWarningDialog::AssignWorking));
    QVERIFY(profileName().contains(QStringLiteral("sRGB")));
    QCOMPARE(window_->activeView()->sample_argb(2, 2), fixture.sample);
    closeActive();
}

void ProfileWarningsTest::missingAssignConvert()
{
    const Fixture fixture = makePsd(QStringLiteral("untagged-convert.psd"), false, false);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(false, true);
    QVERIFY(openWith(fixture.path, true, pictura::ProfileWarningDialog::AssignThenConvert));
    QCOMPARE(window_->activeView()->sample_argb(2, 2), fixture.sample);
    closeActive();
}

void ProfileWarningsTest::missingLeaveUntagged()
{
    const Fixture fixture = makePsd(QStringLiteral("untagged-leave.psd"), false, false);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(false, true);
    QVERIFY(openWith(fixture.path, true, pictura::ProfileWarningDialog::LeaveUntagged));
    QVERIFY(profileName().contains(QStringLiteral("sRGB")));
    QCOMPARE(window_->activeView()->sample_argb(2, 2), fixture.sample);
    closeActive();
}

void ProfileWarningsTest::cancelAborts()
{
    const Fixture fixture = makePsd(QStringLiteral("adobe-cancel.psd"), true, false);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(true, false);
    const int before = window_->documentCount();
    QVERIFY(!openWith(fixture.path, false, pictura::ProfileWarningDialog::UseEmbedded));
    QCOMPARE(window_->documentCount(), before);
}

void ProfileWarningsTest::cmykMismatch()
{
    const Fixture fixture = makePsd(QStringLiteral("cmyk.psd"), false, true);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(true, false);
    const int before = window_->documentCount();
    QVERIFY(openWith(fixture.path, true, pictura::ProfileWarningDialog::ConvertToWorking));
    QCOMPARE(window_->documentCount(), before + 1);
    closeActive();
}

void ProfileWarningsTest::rasterMismatchPreserve()
{
    const QString path = makeJpeg(QStringLiteral("adobe.jpg"), true);
    QVERIFY(!path.isEmpty());
    setAsks(true, false);
    answerNext(true, pictura::ProfileWarningDialog::UseEmbedded);
    QVERIFY(window_->openImagePath(path));
    stopAnswering();
    QVERIFY2(window_->activeView()->open_notice().contains(QStringLiteral("preserved")),
             qPrintable(window_->activeView()->open_notice()));
    closeActive();
}

void ProfileWarningsTest::disabled()
{
    const Fixture fixture = makePsd(QStringLiteral("adobe-silent.psd"), true, false);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(false, false);
    QVERIFY(window_->openPath(fixture.path));
    QVERIFY2(profileName() == QStringLiteral("RGB built-in"), qPrintable(profileName()));
    closeActive();
}

void ProfileWarningsTest::defaultsOff()
{
    const pictura::SessionState state;
    QVERIFY(!state.askMismatchOpen);
    QVERIFY(!state.askMissingOpen);
    const Fixture fixture = makePsd(QStringLiteral("defaults.psd"), true, false);
    QVERIFY(!fixture.path.isEmpty());
    setAsks(false, false);
    QVERIFY(window_->openPath(fixture.path));
    closeActive();
}

void ProfileWarningsTest::preferenceSurvivesRestart()
{
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    pictura::SessionState state;
    state.askMismatchOpen = true;
    state.askMissingOpen = true;
    QVERIFY(pictura::saveSession(state));
    const pictura::SessionState loaded = pictura::loadSession();
    QVERIFY(loaded.askMismatchOpen);
    QVERIFY(loaded.askMissingOpen);
}

void ProfileWarningsTest::notADocumentEdit()
{
    pictura::PictureView view;
    const int history = view.history_count();
    const bool dirty = view.is_dirty();
    pictura::set_color_settings(view, 0, 0, 1, true, true, true, true);
    QCOMPARE(view.history_count(), history);
    QCOMPARE(view.is_dirty(), dirty);
}

QTEST_MAIN(ProfileWarningsTest)
#include "tst_profile_warnings.moc"
