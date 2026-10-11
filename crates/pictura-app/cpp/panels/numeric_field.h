#pragma once

#include <QtCore/QPoint>
#include <QtCore/QString>
#include <QtWidgets/QWidget>

class QLabel;
class QLineEdit;
class QSlider;
class QToolButton;

namespace pictura {

// One numeric input control, configured per site. `decimals == 0` is an integer
// field. `suffix` renders inside the value box. `popup` adds the arrow that
// opens the shared tracking-slider popup.
struct NumericFieldConfig {
    double minimum = 0.0;
    double maximum = 100.0;
    double step = 1.0;
    double page = 10.0;
    int decimals = 0;
    QString suffix;
    bool popup = false;
    // Child object names are `<namePrefix>{Label,Edit,Suffix,Arrow,Popup}`.
    QString namePrefix = QStringLiteral("numeric");
    QString objectName;
    // An integer field (decimals == 0) normally rejects a typed fraction; set
    // this to accept one and render it as an integer unless a decimal is typed.
    // Kept last so positional aggregate initializers stay valid.
    bool allowFractional = false;
};

// A leading label, a left-aligned text editor, and an optional slider popup.
// Pressing and dragging horizontally on the label, the value, or the suffix
// scrubs one step per pixel; Shift scales the step up 10x and Ctrl down 10x.
// `valueChanged` previews user input and `valueCommitted` fires once when the
// edit finishes; `setValue` is a programmatic sync that emits nothing.
class NumericField : public QWidget {
    Q_OBJECT

public:
    NumericField(const QString& label, const NumericFieldConfig& config,
                 QWidget* parent = nullptr);

    double value() const { return value_; }
    void setValue(double value);
    QString labelText() const;
    // Show or hide the in-box suffix (e.g. " px"); the row reflows.
    void setSuffixVisible(bool visible);

signals:
    void valueChanged(double value);
    void valueCommitted(double value);

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void commitEdit();
    void applyUserValue(double value);
    void commitPending();
    void syncSlider();
    void layoutSuffix();
    void showPopup();
    QString formatValue(double value) const;

    NumericFieldConfig config_;
    QLabel* label_ = nullptr;
    QLineEdit* edit_ = nullptr;
    QLabel* suffix_ = nullptr;
    QToolButton* arrow_ = nullptr;
    QWidget* popup_ = nullptr;
    QSlider* slider_ = nullptr;
    double value_ = 0.0;
    bool syncing_ = false;
    bool scrubbing_ = false;
    bool pending_ = false;
    // True when the last typed edit contained a decimal point; an integer field
    // (decimals == 0) then keeps the fraction instead of rounding.
    bool decimalTyped_ = false;
    QPoint scrubOrigin_;
    double scrubStart_ = 0.0;
};

} // namespace pictura
