#pragma once

// Shared ACR panel controls: a label + value on one line with a slider or
// dropdown beneath, plus the panel base. Every panel gets the shared
// `AcrSettings` map and emits `changed()` when a control moves.

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QWidget>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QLabel;
class QSlider;
class QVBoxLayout;

namespace pictura {

class AcrSettings;

// Label + spin box on one line, ramp slider beneath. Values are held as doubles;
// the slider carries `decimals` digits of scale.
class AcrSlider : public QWidget {
    Q_OBJECT

public:
    AcrSlider(const QString& label, double minimum, double maximum, double value,
              int decimals = 0, QWidget* parent = nullptr);

    double value() const;
    void setValue(double value);

signals:
    void valueChanged();

private:
    void handleSlider(int raw);
    void handleSpin(double value);

    int factor_ = 1;
    bool updating_ = false;
    QSlider* slider_ = nullptr;
    QDoubleSpinBox* spin_ = nullptr;
};

// A dropdown item whose stored value is a 4-byte enum code but whose label is
// human text (e.g. `WBal`).
struct AcrEnumItem {
    QString label;
    QByteArray code;
};

// Label + combo on one line.
class AcrCombo : public QWidget {
    Q_OBJECT

public:
    AcrCombo(const QString& label, const QStringList& items, const QString& value,
             QWidget* parent = nullptr);

    QString value() const;
    void setValue(const QString& value);

signals:
    void valueChanged();

private:
    QComboBox* combo_ = nullptr;
};

// A plain checkbox row.
class AcrCheck : public QWidget {
    Q_OBJECT

public:
    explicit AcrCheck(const QString& label, bool checked, QWidget* parent = nullptr);

    void setChecked(bool checked);

signals:
    void toggled(bool checked);

private:
    QCheckBox* box_ = nullptr;
};

// A bold section caption.
QLabel* acrSection(const QString& text, QWidget* parent);

// Base for a tab panel: owns a pointer to the shared settings map and a
// `changed()` signal the dialog listens to.
class AcrPanel : public QWidget {
    Q_OBJECT

public:
    explicit AcrPanel(AcrSettings* settings, QWidget* parent = nullptr);

    // Build a labelled control from `key`, prefilled from the settings (or the
    // supplied default), and keep the settings map in step. The row's
    // objectName is the key, so tests can find it.
    AcrSlider* addSlider(QVBoxLayout* column, const QByteArray& key, const QString& label,
                         double minimum, double maximum, int decimals, double defaultValue = 0.0);
    AcrCombo* addCombo(QVBoxLayout* column, const QByteArray& key, const QString& label,
                       const QStringList& items, const QString& defaultValue = {});
    AcrCheck* addCheck(QVBoxLayout* column, const QByteArray& key, const QString& label,
                       bool defaultValue = false);
    // Like `addCombo`, but the items carry 4-byte enum codes; the combo shows the
    // label and the settings map stores `code`, so an enum key (e.g. `WBal`)
    // round-trips instead of being dropped as a non-code value.
    AcrCombo* addEnumCombo(QVBoxLayout* column, const QByteArray& key, const QString& label,
                           const QList<AcrEnumItem>& items);
    void addSection(QVBoxLayout* column, const QString& text);

signals:
    void changed();

protected:
    AcrSettings* settings_ = nullptr;
};

} // namespace pictura
