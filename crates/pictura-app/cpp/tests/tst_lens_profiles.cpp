#include <QtTest/QtTest>

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>

#include <cmath>
#include <memory>

#include "frame.h"
#include "lens_correction_canvas.h"
#include "lens_correction_dialog.h"
#include "lens_correction_panels.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include "qt_test_support.h"

using pictura::LensAutoPanel;
using pictura::LensCorrectionDialog;

namespace {

// Lens profiles (#932): a synthetic lensfun database (an invented lens with
// invented coefficients, not lensfun's CC BY-SA data) and a user profile.
// lensfun's 3:2 default makes distortion terms scale by hypot(1.5, 1)² = 3.25
// onto the half-diagonal.
const char* kLensfun = R"(<lensdatabase version="1">
  <lens>
    <maker>ACME Cameras</maker>
    <model>ACME Cameras 24-70mm f/2.8</model>
    <cropfactor>1</cropfactor>
    <calibration>
      <distortion model="poly5" focal="24" k1="-0.05" k2="0"/>
      <distortion model="poly5" focal="50" k1="-0.02" k2="0"/>
      <distortion model="poly5" focal="70" k1="0.01" k2="0"/>
      <vignetting model="pa" focal="50" aperture="2.8" distance="10" k1="-0.4" k2="0" k3="0"/>
      <vignetting model="pa" focal="50" aperture="4" distance="10" k1="-0.2" k2="0" k3="0"/>
    </calibration>
  </lens>
</lensdatabase>
)";

const char* kUser = "[profile]\nname = ACME Mine RAW\nmake = ACME Cameras\nmodel = ACME One\n"
                    "raw = yes\ndistortion = -0.03 0 0\n";

const QString kZoom = QStringLiteral("ACME Cameras 24-70mm f/2.8");
const QString kSynthetic = QStringLiteral("ACME 50mm (synthetic test profile)");
const QString kMine = QStringLiteral("ACME Mine RAW");

// The slot of the profile's c2 distortion term and its vignette v1.
constexpr int kSubSlot = 16;
constexpr int kFlagSlot = 17;
constexpr int kC2Slot = 19;
constexpr int kVignetteSlot = 33;

template <typename T> T* child(const QObject* parent, const char* name)
{
    return parent->findChild<T*>(QString::fromLatin1(name));
}

bool writeFile(const QString& path, const char* text)
{
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(text) > 0;
}

QStringList listed(const QWidget* dialog)
{
    QStringList out;
    const auto* list = child<QListWidget>(dialog, "lcProfiles");
    for (int i = 0; i < list->count(); ++i) {
        out.append(list->item(i)->text());
    }
    return out;
}

void selectProfile(const QWidget* dialog, const QString& name)
{
    auto* list = child<QListWidget>(dialog, "lcProfiles");
    for (int i = 0; i < list->count(); ++i) {
        if (list->item(i)->text() == name) {
            list->setCurrentRow(i);
        }
    }
}

// The codec's `metadata.psd` fixture: Make "ACME Cameras", Model "ACME One",
// FocalLength 50 mm, FNumber 2.8, no lens model.
QString metadataFixture()
{
    QString fixture =
        QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/metadata.psd");
    QDir dir(QCoreApplication::applicationDirPath());
    for (int i = 0; i < 6 && fixture.isEmpty(); ++i) {
        const QString candidate =
            dir.filePath(QStringLiteral("crates/pictura-codec/tests/fixtures/metadata.psd"));
        if (QFileInfo::exists(candidate)) {
            fixture = candidate;
        } else if (!dir.cdUp()) {
            break;
        }
    }
    return fixture;
}

} // namespace

class LensProfilesTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void userAndLensfunProfilesAreListed();
    void exifPicksTheClosestSubprofile();
    void subprofileMenuSwitchesTheSubprofile();
    void preferRawReordersTheList();
    void unavailableProfileKeepsItsSettings();
    void onlineProfilesStayDisabled();

private:
    std::unique_ptr<LensCorrectionDialog> showDialog();

    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LensProfilesTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    const QString fixture = metadataFixture();
    if (fixture.isEmpty()) {
        QSKIP("metadata.psd fixture not found");
    }
    // The profiles load once per process, so the environment is set first.
    QVERIFY(writeFile(dir_.filePath(QStringLiteral("db/version_1/acme.xml")), kLensfun));
    QVERIFY(writeFile(dir_.filePath(QStringLiteral("data/pictura/lens-profiles/mine.txt")), kUser));
    qputenv("PICTURA_LENSFUN_DIR", dir_.filePath(QStringLiteral("db")).toUtf8());
    qputenv("XDG_DATA_HOME", dir_.filePath(QStringLiteral("data")).toUtf8());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(window_->openDocumentAtPath(fixture));
    QVERIFY(window_->activeView() != nullptr);
}

std::unique_ptr<LensCorrectionDialog> LensProfilesTest::showDialog()
{
    auto dialog = std::make_unique<LensCorrectionDialog>(window_->activeView());
    dialog->show();
    if (!QTest::qWaitForWindowExposed(dialog.get())) {
        return nullptr;
    }
    return dialog;
}

void LensProfilesTest::userAndLensfunProfilesAreListed()
{
    auto dialog = showDialog();
    QVERIFY(dialog && dialog->ready());
    // Camera-specific profiles match the model; the lensfun lens matches by
    // its maker, ranked after them. Equal matches sort by name.
    QCOMPARE(listed(dialog.get()), (QStringList{kSynthetic, kMine, kZoom}));
    QCOMPARE(child<QListWidget>(dialog.get(), "lcProfiles")->currentItem()->text(), kSynthetic);
    dialog->reject();
}

void LensProfilesTest::exifPicksTheClosestSubprofile()
{
    auto dialog = showDialog();
    QVERIFY(dialog && dialog->ready());
    selectProfile(dialog.get(), kZoom);
    LensAutoPanel* panel = dialog->autoPanel();
    QVERIFY(panel->profileId() >= (1 << 30));
    // The photo is 50 mm at f/2.8: of 24, 50 f/2.8, 50 f/4, and 70 mm.
    QCOMPARE(panel->subProfile(), 1);
    const QList<double> values = dialog->values();
    QCOMPARE(values.size(), 36);
    QCOMPARE(values.at(kSubSlot), 1.0);
    QCOMPARE(values.at(kFlagSlot), 1.0);
    QVERIFY(std::abs(values.at(kC2Slot) - (-0.02 * 3.25)) < 1e-9);
    QVERIFY(std::abs(values.at(kVignetteSlot) - (-0.4)) < 1e-9);
    dialog->accept();
    QCOMPARE(dialog->committed(), values);
}

void LensProfilesTest::subprofileMenuSwitchesTheSubprofile()
{
    auto dialog = showDialog();
    QVERIFY(dialog && dialog->ready());
    selectProfile(dialog.get(), kZoom);
    LensAutoPanel* panel = dialog->autoPanel();
    std::unique_ptr<QMenu> menu(panel->subProfileMenu());
    QStringList labels;
    QList<QAction*> subs;
    for (QAction* action : menu->actions()) {
        if (action->isCheckable()) {
            labels.append(action->text());
            subs.append(action);
        }
    }
    QCOMPARE(labels, (QStringList{QStringLiteral("24 mm"), QStringLiteral("50 mm, f/2.8, 10 m"),
                                  QStringLiteral("50 mm, f/4, 10 m"), QStringLiteral("70 mm")}));
    QVERIFY(subs.at(1)->isChecked());
    const QImage before = dialog->canvas()->previewImage();
    subs.at(0)->trigger();
    QCOMPARE(panel->subProfile(), 0);
    const QList<double> values = dialog->values();
    QCOMPARE(values.at(kSubSlot), 0.0);
    QVERIFY(std::abs(values.at(kC2Slot) - (-0.05 * 3.25)) < 1e-9);
    QCOMPARE(values.at(kVignetteSlot), 0.0);
    QVERIFY(dialog->canvas()->previewImage() != before);
    dialog->reject();
}

void LensProfilesTest::preferRawReordersTheList()
{
    {
        auto dialog = showDialog();
        QVERIFY(dialog && dialog->ready());
        auto* preferRaw = child<QAction>(dialog.get(), "lcPreferRaw");
        QVERIFY(preferRaw && preferRaw->isCheckable() && !preferRaw->isChecked());
        preferRaw->setChecked(true);
        QCOMPARE(listed(dialog.get()), (QStringList{kMine, kSynthetic, kZoom}));
        dialog->reject();
    }
    {
        // The choice holds for the session.
        auto dialog = showDialog();
        QVERIFY(dialog && dialog->ready());
        auto* preferRaw = child<QAction>(dialog.get(), "lcPreferRaw");
        QVERIFY(preferRaw->isChecked());
        QCOMPARE(listed(dialog.get()).first(), kMine);
        preferRaw->setChecked(false);
        QCOMPARE(listed(dialog.get()), (QStringList{kSynthetic, kMine, kZoom}));
        dialog->reject();
    }
}

void LensProfilesTest::unavailableProfileKeepsItsSettings()
{
    auto dialog = showDialog();
    QVERIFY(dialog && dialog->ready());
    // A profile id nothing installed has, with its stored coefficients.
    const QList<double> stored = {1, 0, 1, 1, 1, 1234567, 0, 0, 0, 0, 0, 50, 0, 0, 0, 100,
                                  2, 1, 0, -0.05, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0,
                                  -0.2, 0, 0};
    QVERIFY(dialog->load(stored));
    QCOMPARE(child<QListWidget>(dialog.get(), "lcProfiles")->currentItem()->text(),
             QStringLiteral("Profile unavailable"));
    QVERIFY(child<QCheckBox>(dialog.get(), "lcGeometric")->isEnabled());
    QVERIFY(child<QCheckBox>(dialog.get(), "lcVignette")->isEnabled());
    QCOMPARE(dialog->values(), stored);
    dialog->accept();
    QCOMPARE(dialog->committed(), stored);
}

void LensProfilesTest::onlineProfilesStayDisabled()
{
    auto dialog = showDialog();
    QVERIFY(dialog && dialog->ready());
    selectProfile(dialog.get(), kZoom);
    auto* online = child<QPushButton>(dialog.get(), "lcSearchOnline");
    QVERIFY(!online->isEnabled());
    QVERIFY(online->toolTip().contains(QStringLiteral("No online lens profile service")));
    std::unique_ptr<QMenu> menu(dialog->autoPanel()->subProfileMenu());
    auto* save = child<QAction>(menu.get(), "lcSaveOnline");
    QVERIFY(save != nullptr);
    QCOMPARE(save->text(), QStringLiteral("Save Online Profile Locally"));
    QVERIFY(!save->isEnabled());
    dialog->reject();
}

QTEST_MAIN(LensProfilesTest)
#include "tst_lens_profiles.moc"
