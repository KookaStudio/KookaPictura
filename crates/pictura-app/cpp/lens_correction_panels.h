#pragma once

// The Lens Correction dialog's two tabs. Auto Correction holds the slots the
// profile path uses (the Correction checkboxes, Auto Scale Image, Edge, and
// the chosen profile) with the Search Criteria combos and the Lens Profiles
// list; Custom holds the Settings menu and the manual sliders (Remove
// Distortion, the three fringes, Vignette, and Transform). Each reports its
// part of the `lens-correction` slots in order.
// ponytail: no Prefer RAW Profiles, sub-profiles, Search Online, Save /
// Load Settings, or Set Lens Default.

#include <QtCore/QList>
#include <QtWidgets/QWidget>

#include <functional>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QListWidget;
class QPushButton;

namespace pictura {

class AngleDial;

class LensAutoPanel : public QWidget {
    Q_OBJECT

public:
    static constexpr int kSlots = 6;

    // `ready` is whether the Rust session opened; it supplies the EXIF and
    // the installed profiles.
    explicit LensAutoPanel(bool ready, QWidget* parent = nullptr);

    QList<double> values() const;
    // Restore the first `kSlots` slots; false when the profile they name is
    // not installed.
    bool setValues(const QList<double>& values);
    // The selected profile's id, 0 for none.
    int profileId() const;

    std::function<void()> changed;

private:
    void fillModels();
    void fillLenses();
    void fillProfiles();
    void updateCorrections();
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
