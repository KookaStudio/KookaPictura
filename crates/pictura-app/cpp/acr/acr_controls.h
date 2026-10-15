#pragma once

// Shared ACR panel controls: Photoshop-style adjustment sliders (a name and a
// value field over a house-pointer track), range sliders, dropdowns,
// checkboxes, section separators, and the panel base. Every panel gets the
// shared `AcrSettings` map and emits `changed()` when a control moves.

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtCore/QString>
#include <QtGui/QColor>
#include <QtWidgets/QWidget>

class QCheckBox;
class QComboBox;
class QDoubleSpinBox;
class QEvent;
class QLabel;
class QMouseEvent;
class QPaintEvent;
class QVBoxLayout;

namespace pictura {

class AcrSettings;

// A dropdown item whose stored value is a 4-byte enum code but whose label is
// human text (e.g. `WBal`).
struct AcrEnumItem {
    QString label;
    QByteArray code;
};

// The ACR slider: a name and a value on one line over a track. The track fills
// from its neutral value to the pointer and may carry a colour ramp; the
// pointer is house-shaped. A click or drag on the track jumps to that value,
// and dragging the name scrubs the value.
class AcrSlider : public QWidget {
    Q_OBJECT

public:
    AcrSlider(const QString& label, double minimum, double maximum, double value,
              int decimals = 0, QWidget* parent = nullptr);

    double value() const;
    void setValue(double value);
    void setRamp(const QList<QColor>& stops);

signals:
    void valueChanged();

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    QSize sizeHint() const override;

private:
    void setFromX(int x);
    double valueForX(int x) const;
    int xForValue(double value) const;
    double originFraction() const;

    double min_ = 0.0;
    double max_ = 100.0;
    double value_ = 0.0;
    int decimals_ = 0;
    QList<QColor> ramp_;
    bool dragging_ = false;
    bool scrubbing_ = false;
    bool updating_ = false;
    int scrubStartX_ = 0;
    double scrubStartValue_ = 0.0;
    QLabel* label_ = nullptr;
    QDoubleSpinBox* field_ = nullptr;
};

// A slider with two pointers (a hue range). The readout is a `lo / hi` label.
class AcrRangeSlider : public QWidget {
    Q_OBJECT

public:
    AcrRangeSlider(const QString& label, double minimum, double maximum, double lo, double hi,
                   int decimals = 0, QWidget* parent = nullptr);

    double low() const;
    double high() const;
    void setValues(double lo, double hi);
    void setRamp(const QList<QColor>& stops);

signals:
    void valueChanged();

protected:
    void paintEvent(QPaintEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    QSize sizeHint() const override;

private:
    int xForValue(double value) const;
    double valueForX(int x) const;

    double min_ = 0.0;
    double max_ = 360.0;
    double lo_ = 0.0;
    double hi_ = 360.0;
    int decimals_ = 0;
    QList<QColor> ramp_;
    int dragging_ = -1;
    QLabel* label_ = nullptr;
    QLabel* value_ = nullptr;
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

// A bold, centred section caption followed by a rule, drawn by `addSection`.
QLabel* acrSection(const QString& text, QWidget* parent);

// Base for a tab panel: owns a pointer to the shared settings map and a
// `changed()` signal the dialog listens to.
class AcrPanel : public QWidget {
    Q_OBJECT

public:
    explicit AcrPanel(AcrSettings* settings, QWidget* parent = nullptr);

    AcrSlider* addSlider(QVBoxLayout* column, const QByteArray& key, const QString& label,
                         double minimum, double maximum, int decimals, double defaultValue = 0.0,
                         const QList<QColor>& ramp = {});
    AcrRangeSlider* addRangeSlider(QVBoxLayout* column, const QByteArray& keyLo,
                                   const QByteArray& keyHi, const QString& label, double minimum,
                                   double maximum, int decimals, double defaultLo, double defaultHi,
                                   const QList<QColor>& ramp = {});
    AcrCombo* addCombo(QVBoxLayout* column, const QByteArray& key, const QString& label,
                       const QStringList& items, const QString& defaultValue = {});
    AcrCombo* addEnumCombo(QVBoxLayout* column, const QByteArray& key, const QString& label,
                           const QList<AcrEnumItem>& items);
    AcrCheck* addCheck(QVBoxLayout* column, const QByteArray& key, const QString& label,
                       bool defaultValue = false);
    void addSection(QVBoxLayout* column, const QString& text);
    void addSeparator(QVBoxLayout* column);

signals:
    void changed();

protected:
    AcrSettings* settings_ = nullptr;
};

} // namespace pictura
