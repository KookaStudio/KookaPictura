#pragma once

// The Lens Correction dialog's two tabs. Auto Correction holds the slots the
// profile path uses (the Correction checkboxes, Auto Scale Image, Edge, and
// the chosen profile) with the Search Criteria combos (filled from the camera
// and lens catalogues, the lenses filtered by the photo's focal length) and
// the Lens Profiles list (with the sub-profile menu on right-click) and the
// profile's resolved coefficients; Custom holds the Settings menu and the
// manual sliders (Remove Distortion, the three fringes, Vignette, and
// Transform). Each reports its part of the `lens-correction` slots in order.
// Search Online and Save Online Profile Locally stay disabled: there is no
// online profile service (#932).
// ponytail: no Save / Load Settings or Set Lens Default (#933).

#include <QtCore/QList>
#include <QtWidgets/QWidget>

#include <functional>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QLabel;
class QAction;
class QListWidget;
class QMenu;
class QPushButton;
class QToolButton;

namespace pictura {

class AngleDial;

class LensAutoPanel : public QWidget {
    Q_OBJECT

public:
    // The dialog's first slots, and the profile slots after the Custom
    // tab's: the sub-profile index, the coefficients flag, and the
    // coefficients.
    static constexpr int kSlots = 6;
    static constexpr int kProfileSlots = 20;

    // `ready` is whether the Rust session opened; it supplies the EXIF and
    // the installed profiles.
    explicit LensAutoPanel(bool ready, QWidget* parent = nullptr);

    QList<double> values() const;
    QList<double> profileValues() const;
    // Restore the first `kSlots` slots and the profile slots. Stored
    // coefficients are kept as they are, and a profile that is not installed
    // is listed as "Profile unavailable"; false when the slots carry no
    // coefficients and name a profile that is not installed.
    bool setValues(const QList<double>& values, const QList<double>& profile);
    // The selected profile's id, 0 for none, and its sub-profile index.
    int profileId() const;
    int subProfile() const;
    // The right-click menu: the selected profile's sub-profiles, the current
    // one checked, then a disabled Save Online Profile Locally. The caller
    // owns it.
    QMenu* subProfileMenu();

    std::function<void()> changed;

private:
    void fillModels();
    void fillLenses();
    void fillProfiles();
    void updateCorrections();
    // Resolve the selected profile's sub-profile for the criteria camera.
    void resolveProfile();
    void selectSubProfile(int sub);
    void emitChanged();

    bool gray_ = false;
    QCheckBox* geometric_ = nullptr;
    QCheckBox* chromatic_ = nullptr;
    QCheckBox* vignette_ = nullptr;
    QCheckBox* autoScale_ = nullptr;
    QComboBox* edge_ = nullptr;
    QComboBox* make_ = nullptr;
    QComboBox* model_ = nullptr;
    QComboBox* lens_ = nullptr;
    QListWidget* profiles_ = nullptr;
    QPushButton* searchOnline_ = nullptr;
    QToolButton* searchMenu_ = nullptr;
    QAction* preferRaw_ = nullptr;
    int sub_ = 0;
    // The selected profile's slots; empty with no profile.
    QList<double> profileSlots_;
    // The catalogues' line about the chosen camera and lens.
    QLabel* info_ = nullptr;
};

class LensCustomPanel : public QWidget {
    Q_OBJECT

public:
    static constexpr int kSlots = 10;
    enum Setting { DefaultCorrection, PreviousConversion, Custom };

    explicit LensCustomPanel(bool gray, QWidget* parent = nullptr);

    QList<double> values() const;
    // Restore the last `kSlots` slots without touching the Settings menu.
    void setValues(const QList<double>& values);

    double distortion() const;
    void setDistortion(double value);
    double angle() const;
    void setAngle(double degrees);
    int setting() const;
    // Apply a Settings menu item: Default Correction or Previous Conversion
    // reset or restore the distortion, fringe, and vignette sliders; the
    // transform is per image and stays (CS6 does not save it).
    void applySetting(int setting);

    std::function<void()> changed;

private:
    QDoubleSpinBox* addSlider(QWidget* box, const QString& objectName, const QString& label,
                              double minimum, double maximum, double value, int decimals,
                              const QString& unit = QString());
    void edited();

    bool loading_ = false;
    QComboBox* settings_ = nullptr;
    QList<QDoubleSpinBox*> boxes_; // slot order
    AngleDial* dial_ = nullptr;
};

} // namespace pictura
