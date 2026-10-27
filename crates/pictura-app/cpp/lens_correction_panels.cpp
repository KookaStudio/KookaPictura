#include "lens_correction_panels.h"
#include "panels/angle_dial.h"
#include "panels/jump_slider.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QActionGroup>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>

#include "pictura_app/src/cxxqt_object/filter_tools/lens_correction.cxxqt.h"

namespace pictura {

namespace {

const QString kNoMatch = QStringLiteral("No matching profiles found");
const QString kUnavailable = QStringLiteral("Profile unavailable");
const QString kNoOnline =
    QStringLiteral("No online lens profile service is available. Install lensfun's database "
                   "(lensfun-data) for real profiles.");

// The profile slots with no profile: no sub-profile, no coefficients, then the
// identity model.
const QList<double> kNoProfile = {0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0};

// Prefer RAW Profiles holds for the rest of the session.
bool gPreferRaw = false;

// The Custom slots' opening values, in slot order.
constexpr double kCustomDefaults[LensCustomPanel::kSlots] = {0, 0, 0, 0, 0, 50, 0, 0, 0, 100};
// The slots Settings covers: distortion, the fringes, and the vignette.
constexpr int kSettingSlots = 6;

// Fill `combo` with `items`, keeping (or adding) `keep`; no criterion when
// `keep` is empty.
void fillCombo(QComboBox* combo, QStringList items, const QString& keep)
{
    const QSignalBlocker block(combo);
    if (!keep.isEmpty() && !items.contains(keep, Qt::CaseInsensitive)) {
        items.prepend(keep);
    }
    combo->clear();
    combo->addItems(items);
    combo->setCurrentIndex(keep.isEmpty() ? -1 : combo->findText(keep, Qt::MatchFixedString));
}

} // namespace

LensAutoPanel::LensAutoPanel(bool ready, QWidget* parent) : QWidget(parent)
{
    setObjectName(QStringLiteral("lcAutoPanel"));
    gray_ = ready && lc_grayscale();
    auto* layout = new QVBoxLayout(this);

    auto* correction = new QGroupBox(QStringLiteral("Correction"), this);
    auto* correctionLayout = new QVBoxLayout(correction);
    auto check = [&](const QString& name, const QString& text, bool on) {
        auto* box = new QCheckBox(text, correction);
        box->setObjectName(name);
        box->setChecked(on);
        correctionLayout->addWidget(box);
        connect(box, &QCheckBox::toggled, this, &LensAutoPanel::emitChanged);
        return box;
    };
    geometric_ = check(QStringLiteral("lcGeometric"), QStringLiteral("Geometric Distortion"), true);
    chromatic_ = check(QStringLiteral("lcChromatic"), QStringLiteral("Chromatic Aberration"), false);
    vignette_ = check(QStringLiteral("lcVignette"), QStringLiteral("Vignette"), false);
    geometric_->setToolTip(QStringLiteral("Correct the profile's barrel or pincushion distortion."));
    chromatic_->setToolTip(QStringLiteral("Correct the profile's color fringing."));
    vignette_->setToolTip(QStringLiteral("Correct the profile's darkened corners."));
    autoScale_ = check(QStringLiteral("lcAutoScale"), QStringLiteral("Auto Scale Image"), true);
    autoScale_->setToolTip(
        QStringLiteral("Scale the image so the profile correction leaves no blank area."));
    edge_ = new QComboBox(correction);
    edge_->setObjectName(QStringLiteral("lcEdge"));
    edge_->addItems({QStringLiteral("Edge Extension"), QStringLiteral("Transparency"),
                     QStringLiteral("Black Color"), QStringLiteral("White Color")});
    edge_->setCurrentIndex(1);
    edge_->setToolTip(QStringLiteral("Edge: how blank areas are filled."));
    connect(edge_, &QComboBox::currentIndexChanged, this, &LensAutoPanel::emitChanged);
    auto* edgeRow = new QFormLayout;
    edgeRow->addRow(QStringLiteral("Edge:"), edge_);
    correctionLayout->addLayout(edgeRow);
    layout->addWidget(correction);

    auto* search = new QGroupBox(QStringLiteral("Search Criteria"), this);
    auto* searchLayout = new QFormLayout(search);
    auto combo = [&](const QString& name, const QString& placeholder) {
        auto* box = new QComboBox(search);
        box->setObjectName(name);
        box->setPlaceholderText(placeholder);
        box->setMinimumContentsLength(16);
        return box;
    };
    make_ = combo(QStringLiteral("lcMake"), QStringLiteral("Choose a camera make"));
    model_ = combo(QStringLiteral("lcModel"), QStringLiteral("Choose a camera model"));
    lens_ = combo(QStringLiteral("lcLens"), QStringLiteral("Choose a lens model"));
    searchLayout->addRow(QStringLiteral("Camera Make:"), make_);
    searchLayout->addRow(QStringLiteral("Camera Model:"), model_);
    searchLayout->addRow(QStringLiteral("Lens Model:"), lens_);
    // The Search Criteria pop-up; the list sorts by sensor size unless raw
    // profiles are preferred.
    searchMenu_ = new QToolButton(search);
    searchMenu_->setObjectName(QStringLiteral("lcSearchMenu"));
    searchMenu_->setText(QStringLiteral("Search Options"));
    searchMenu_->setPopupMode(QToolButton::InstantPopup);
    searchMenu_->setToolTip(
        QStringLiteral("Search Criteria: profiles are sorted by sensor size; prefer raw profiles."));
    auto* searchOptions = new QMenu(searchMenu_);
    preferRaw_ = searchOptions->addAction(QStringLiteral("Prefer RAW Profiles"));
    preferRaw_->setObjectName(QStringLiteral("lcPreferRaw"));
    preferRaw_->setCheckable(true);
    preferRaw_->setChecked(gPreferRaw);
    searchMenu_->setMenu(searchOptions);
    searchLayout->addRow(searchMenu_);
    info_ = new QLabel(search);
    info_->setObjectName(QStringLiteral("lcCriteriaInfo"));
    info_->setWordWrap(true);
    searchLayout->addRow(info_);
    layout->addWidget(search);

    auto* profileBox = new QGroupBox(QStringLiteral("Lens Profiles"), this);
    auto* profileLayout = new QVBoxLayout(profileBox);
    profiles_ = new QListWidget(profileBox);
    profiles_->setObjectName(QStringLiteral("lcProfiles"));
    profiles_->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(profiles_, &QWidget::customContextMenuRequested, this, [this](const QPoint& at) {
        QMenu* menu = subProfileMenu();
        menu->exec(profiles_->viewport()->mapToGlobal(at));
        delete menu;
    });
    profileLayout->addWidget(profiles_);
    // The button stays for parity; there is no online profile source.
    searchOnline_ = new QPushButton(QStringLiteral("Search Online"), profileBox);
    searchOnline_->setObjectName(QStringLiteral("lcSearchOnline"));
    searchOnline_->setEnabled(false);
    searchOnline_->setToolTip(kNoOnline);
    profileLayout->addWidget(searchOnline_, 0, Qt::AlignRight);
    layout->addWidget(profileBox, 1);

    // The criteria open on what EXIF says.
    QStringList exif;
    if (ready) {
        for (const QString& s : lc_exif()) {
            exif.append(s);
        }
    }
    fillCombo(make_, lc_makes(), exif.value(0));
    fillCombo(model_, lc_models(make_->currentText()), exif.value(1));
    fillCombo(lens_, lc_lenses(make_->currentText(), model_->currentText()),
              exif.value(2));
    fillProfiles();
    connect(make_, &QComboBox::currentIndexChanged, this, [this] {
        fillModels();
        fillLenses();
        fillProfiles();
        emitChanged();
    });
    connect(model_, &QComboBox::currentIndexChanged, this, [this] {
        fillLenses();
        fillProfiles();
        emitChanged();
    });
    connect(lens_, &QComboBox::currentIndexChanged, this, [this] {
        fillProfiles();
        emitChanged();
    });
    connect(profiles_, &QListWidget::currentRowChanged, this, [this] {
        sub_ = lc_auto_subprofile(profileId());
        resolveProfile();
        updateCorrections();
        emitChanged();
    });
    connect(preferRaw_, &QAction::toggled, this, [this](bool on) {
        gPreferRaw = on;
        fillProfiles();
        emitChanged();
    });
}

void LensAutoPanel::fillModels()
{
    fillCombo(model_, lc_models(make_->currentText()), QString());
}

void LensAutoPanel::fillLenses()
{
    fillCombo(lens_, lc_lenses(make_->currentText(), model_->currentText()),
              QString());
}

void LensAutoPanel::fillProfiles()
{
    const int keep = profileId();
    const QSignalBlocker block(profiles_);
    profiles_->clear();
    for (int32_t id : lc_match_profiles(make_->currentText(), model_->currentText(),
                                        lens_->currentText(), preferRaw_->isChecked())) {
        auto* item = new QListWidgetItem(QString(lc_profile_name(id)), profiles_);
        item->setData(Qt::UserRole, id);
    }
    if (profiles_->count() == 0) {
        auto* item = new QListWidgetItem(kNoMatch, profiles_);
        item->setFlags(Qt::NoItemFlags);
        item->setTextAlignment(Qt::AlignCenter);
        item->setData(Qt::UserRole, 0);
    } else {
        // The best match is chosen, as CS6 does, unless the old choice stays.
        int row = 0;
        for (int i = 0; i < profiles_->count(); ++i) {
            if (keep != 0 && profiles_->item(i)->data(Qt::UserRole).toInt() == keep) {
                row = i;
            }
        }
        profiles_->setCurrentRow(row);
    }
    // A new profile takes the sub-profile EXIF suits; a kept one keeps its
    // choice, re-resolved for the camera.
    if (profileId() != keep) {
        sub_ = lc_auto_subprofile(profileId());
    }
    resolveProfile();
    updateCorrections();
    info_->setText(QString(
        lc_criteria_info(make_->currentText(), model_->currentText(), lens_->currentText())));
}

void LensAutoPanel::updateCorrections()
{
    const bool profile = profileId() != 0;
    geometric_->setEnabled(profile);
    chromatic_->setEnabled(profile && !gray_);
    vignette_->setEnabled(profile);
}

void LensAutoPanel::resolveProfile()
{
    profileSlots_.clear();
    const int id = profileId();
    if (id == 0) {
        return;
    }
    for (double v : lc_profile_slots(id, sub_, make_->currentText(), model_->currentText())) {
        profileSlots_.append(v);
    }
}

void LensAutoPanel::selectSubProfile(int sub)
{
    sub_ = sub;
    resolveProfile();
    emitChanged();
}

int LensAutoPanel::profileId() const
{
    const QListWidgetItem* item = profiles_->currentItem();
    return item ? item->data(Qt::UserRole).toInt() : 0;
}

int LensAutoPanel::subProfile() const
{
    return sub_;
}

QMenu* LensAutoPanel::subProfileMenu()
{
    auto* menu = new QMenu(this);
    menu->setObjectName(QStringLiteral("lcSubProfileMenu"));
    auto* group = new QActionGroup(menu);
    const QStringList subs = lc_subprofiles(profileId());
    for (int i = 0; i < subs.size(); ++i) {
        QAction* action = menu->addAction(subs.at(i));
        action->setCheckable(true);
        action->setChecked(i == sub_);
        group->addAction(action);
        connect(action, &QAction::triggered, this, [this, i] { selectSubProfile(i); });
    }
    if (subs.isEmpty()) {
        menu->addAction(QStringLiteral("No sub-profiles"))->setEnabled(false);
    }
    menu->addSeparator();
    QAction* save = menu->addAction(QStringLiteral("Save Online Profile Locally"));
    save->setObjectName(QStringLiteral("lcSaveOnline"));
    save->setEnabled(false);
    save->setToolTip(kNoOnline);
    return menu;
}

QList<double> LensAutoPanel::values() const
{
    return {geometric_->isChecked() ? 1.0 : 0.0,
            chromatic_->isChecked() && !gray_ ? 1.0 : 0.0,
            vignette_->isChecked() ? 1.0 : 0.0,
            autoScale_->isChecked() ? 1.0 : 0.0,
            static_cast<double>(edge_->currentIndex()),
            static_cast<double>(profileId())};
}

QList<double> LensAutoPanel::profileValues() const
{
    return profileId() != 0 && profileSlots_.size() == kProfileSlots ? profileSlots_ : kNoProfile;
}

bool LensAutoPanel::setValues(const QList<double>& values, const QList<double>& profile)
{
    if (values.size() < kSlots) {
        return false;
    }
    const int id = static_cast<int>(std::lround(values.at(5)));
    const bool stored = profile.size() == kProfileSlots && profile.at(1) >= 0.5;
    const QString name = id != 0 ? QString(lc_profile_name(id)) : QString();
    if (id != 0 && !stored && name.isEmpty()) {
        return false;
    }
    const QSignalBlocker a(geometric_);
    const QSignalBlocker b(chromatic_);
    const QSignalBlocker c(vignette_);
    const QSignalBlocker d(autoScale_);
    const QSignalBlocker e(edge_);
    geometric_->setChecked(values.at(0) >= 0.5);
    chromatic_->setChecked(values.at(1) >= 0.5);
    vignette_->setChecked(values.at(2) >= 0.5);
    autoScale_->setChecked(values.at(3) >= 0.5);
    edge_->setCurrentIndex(std::clamp(static_cast<int>(std::lround(values.at(4))), 0, 3));
    auto restoreProfile = [&] {
        if (id == 0) {
            return;
        }
        if (stored) {
            sub_ = static_cast<int>(std::lround(profile.at(0)));
            profileSlots_ = profile;
        } else {
            sub_ = lc_auto_subprofile(id);
            resolveProfile();
        }
    };
    if (id == profileId()) {
        restoreProfile();
        return true;
    }
    // Show every profile so the restored one is listed.
    {
        const QSignalBlocker f(make_);
        const QSignalBlocker g(model_);
        const QSignalBlocker h(lens_);
        make_->setCurrentIndex(-1);
        fillModels();
        fillLenses();
    }
    const QSignalBlocker block(profiles_);
    profiles_->clear();
    if (id == 0) {
        fillProfiles();
        profiles_->setCurrentRow(-1);
    } else {
        auto* item = new QListWidgetItem(name.isEmpty() ? kUnavailable : name, profiles_);
        item->setData(Qt::UserRole, id);
        profiles_->setCurrentRow(0);
    }
    restoreProfile();
    updateCorrections();
    return true;
}

void LensAutoPanel::emitChanged()
{
    if (changed) {
        changed();
    }
}

LensCustomPanel::LensCustomPanel(bool gray, QWidget* parent) : QWidget(parent)
{
    setObjectName(QStringLiteral("lcCustomPanel"));
    auto* layout = new QVBoxLayout(this);
    settings_ = new QComboBox(this);
    settings_->setObjectName(QStringLiteral("lcSettings"));
    settings_->addItems({QStringLiteral("Default Correction"), QStringLiteral("Previous Conversion"),
                         QStringLiteral("Custom")});
    if (auto* model = qobject_cast<QStandardItemModel*>(settings_->model())) {
        // Previous Conversion needs an earlier OK.
        model->item(PreviousConversion)->setEnabled(!lc_previous().empty());
    }
    connect(settings_, &QComboBox::activated, this, &LensCustomPanel::applySetting);
    auto* settingsRow = new QFormLayout;
    settingsRow->addRow(QStringLiteral("Settings:"), settings_);
    layout->addLayout(settingsRow);

    auto group = [&](const QString& title) {
        auto* box = new QGroupBox(title, this);
        new QVBoxLayout(box);
        layout->addWidget(box);
        return box;
    };
    QGroupBox* geometry = group(QStringLiteral("Geometric Distortion"));
    addSlider(geometry, QStringLiteral("lcDistortion"), QStringLiteral("Remove Distortion"), -100,
              100, 0, 2)
        ->setToolTip(QStringLiteral(
            "Remove Distortion: positive values correct barrel distortion, negative values "
            "pincushion distortion."));
    QGroupBox* fringe = group(QStringLiteral("Chromatic Aberration"));
    const QString fringes[] = {QStringLiteral("Red/Cyan"), QStringLiteral("Green/Magenta"),
                               QStringLiteral("Blue/Yellow")};
    const QString fringeNames[] = {QStringLiteral("lcFringeRed"), QStringLiteral("lcFringeGreen"),
                                   QStringLiteral("lcFringeBlue")};
    for (int i = 0; i < 3; ++i) {
        addSlider(fringe, fringeNames[i], QStringLiteral("Fix %1 Fringe").arg(fringes[i]), -100,
                  100, 0, 2)
            ->setToolTip(QStringLiteral("Fix %1 Fringe: scales the channel against the others "
                                        "to remove color fringing.")
                             .arg(fringes[i]));
    }
    // Grayscale has no colour fringe to fix.
    fringe->setEnabled(!gray);
    QGroupBox* vignette = group(QStringLiteral("Vignette"));
    addSlider(vignette, QStringLiteral("lcVignetteAmount"), QStringLiteral("Amount"), -100, 100,
              0, 0)
        ->setToolTip(QStringLiteral("Vignette Amount: darken or lighten the image's corners."));
    auto* ends = new QHBoxLayout;
    ends->addWidget(new QLabel(QStringLiteral("darken"), vignette));
    ends->addStretch(1);
    ends->addWidget(new QLabel(QStringLiteral("lighten"), vignette));
    static_cast<QVBoxLayout*>(vignette->layout())->addLayout(ends);
    addSlider(vignette, QStringLiteral("lcVignetteMidpoint"), QStringLiteral("Midpoint"), 0, 100,
              50, 0)
        ->setToolTip(
            QStringLiteral("Vignette Midpoint: lower values widen the area the Amount affects."));
    QGroupBox* transform = group(QStringLiteral("Transform"));
    addSlider(transform, QStringLiteral("lcVertical"), QStringLiteral("Vertical Perspective"), -100,
              100, 0, 0)
        ->setToolTip(QStringLiteral("Vertical Perspective: corrects tilting the camera up or "
                                    "down, making vertical lines parallel."));
    addSlider(transform, QStringLiteral("lcHorizontal"), QStringLiteral("Horizontal Perspective"),
              -100, 100, 0, 0)
        ->setToolTip(QStringLiteral("Horizontal Perspective: corrects turning the camera left or "
                                    "right, making horizontal lines parallel."));

    // Angle: CS6's dial points up at 0° and turns clockwise with the value.
    auto* angle = new QDoubleSpinBox(transform);
    angle->setObjectName(QStringLiteral("lcAngle"));
    angle->setRange(0.0, 360.0);
    angle->setDecimals(2);
    angle->setWrapping(true);
    angle->setSuffix(QStringLiteral("°"));
    angle->setToolTip(QStringLiteral("Angle: rotates the image to correct camera tilt."));
    dial_ = new AngleDial(transform);
    dial_->setObjectName(QStringLiteral("lcAngleDial"));
    dial_->setFixedSize(32, 32);
    dial_->setAngle(90.0);
    connect(dial_, &AngleDial::angleChanged, this, [angle](double d) {
        angle->setValue(std::fmod(450.0 - d, 360.0));
    });
    connect(angle, &QDoubleSpinBox::valueChanged, this, [this](double v) {
        const QSignalBlocker block(dial_);
        dial_->setAngle(std::fmod(450.0 - v, 360.0));
        edited();
    });
    auto* angleRow = new QHBoxLayout;
    angleRow->addWidget(new QLabel(QStringLiteral("Angle:"), transform));
    angleRow->addWidget(dial_);
    angleRow->addStretch(1);
    angleRow->addWidget(angle);
    static_cast<QVBoxLayout*>(transform->layout())->addLayout(angleRow);
    boxes_.append(angle);
    addSlider(transform, QStringLiteral("lcScale"), QStringLiteral("Scale"), 50, 150, 100, 0,
              QStringLiteral("%"))
        ->setToolTip(QStringLiteral(
            "Scale: zooms the image to fill blank areas; the pixel dimensions stay."));
    layout->addStretch(1);
}

QDoubleSpinBox* LensCustomPanel::addSlider(QWidget* box, const QString& objectName,
                                           const QString& label, double minimum, double maximum,
                                           double value, int decimals, const QString& unit)
{
    auto* spin = new QDoubleSpinBox(box);
    spin->setObjectName(objectName);
    spin->setRange(minimum, maximum);
    spin->setDecimals(decimals);
    spin->setValue(value);
    const double scale = std::pow(10.0, decimals);
    auto* slider = new JumpSlider(Qt::Horizontal, box);
    slider->setObjectName(objectName + QStringLiteral("Slider"));
    slider->setRange(static_cast<int>(std::lround(minimum * scale)),
                     static_cast<int>(std::lround(maximum * scale)));
    slider->setValue(static_cast<int>(std::lround(value * scale)));
    connect(spin, &QDoubleSpinBox::valueChanged, this, [this, slider, scale](double v) {
        const QSignalBlocker block(slider);
        slider->setValue(static_cast<int>(std::lround(v * scale)));
        edited();
    });
    connect(slider, &QSlider::valueChanged, spin, [spin, scale](int v) { spin->setValue(v / scale); });
    auto* top = new QHBoxLayout;
    auto* caption = new QLabel(label, box);
    caption->setBuddy(spin);
    top->addWidget(caption);
    top->addStretch(1);
    top->addWidget(spin);
    if (!unit.isEmpty()) {
        top->addWidget(new QLabel(unit, box));
    }
    auto* column = static_cast<QVBoxLayout*>(box->layout());
    column->addLayout(top);
    column->addWidget(slider);
    boxes_.append(spin);
    return spin;
}

QList<double> LensCustomPanel::values() const
{
    QList<double> out;
    for (const QDoubleSpinBox* box : boxes_) {
        out.append(box->value());
    }
    return out;
}

void LensCustomPanel::setValues(const QList<double>& values)
{
    loading_ = true;
    for (int i = 0; i < boxes_.size() && i < values.size(); ++i) {
        boxes_.at(i)->setValue(values.at(i));
    }
    loading_ = false;
}

double LensCustomPanel::distortion() const
{
    return boxes_.at(0)->value();
}

void LensCustomPanel::setDistortion(double value)
{
    boxes_.at(0)->setValue(value);
}

double LensCustomPanel::angle() const
{
    return boxes_.at(8)->value();
}

void LensCustomPanel::setAngle(double degrees)
{
    boxes_.at(8)->setValue(std::fmod(std::fmod(degrees, 360.0) + 360.0, 360.0));
}

int LensCustomPanel::setting() const
{
    return settings_->currentIndex();
}

void LensCustomPanel::applySetting(int setting)
{
    QList<double> restore;
    if (setting == DefaultCorrection) {
        restore = QList<double>(kCustomDefaults, kCustomDefaults + kSettingSlots);
    } else if (setting == PreviousConversion) {
        const ::rust::Vec<double> previous = lc_previous();
        constexpr size_t first = LensAutoPanel::kSlots;
        if (previous.size() < first + kSettingSlots) {
            return;
        }
        for (size_t i = 0; i < kSettingSlots; ++i) {
            restore.append(previous[first + i]);
        }
    }
    {
        const QSignalBlocker block(settings_);
        settings_->setCurrentIndex(setting);
    }
    if (restore.isEmpty()) {
        return;
    }
    setValues(restore);
    if (changed) {
        changed();
    }
}

void LensCustomPanel::edited()
{
    if (loading_) {
        return;
    }
    {
        const QSignalBlocker block(settings_);
        settings_->setCurrentIndex(Custom);
    }
    if (changed) {
        changed();
    }
}

} // namespace pictura

#include "moc_lens_correction_panels.cpp"
