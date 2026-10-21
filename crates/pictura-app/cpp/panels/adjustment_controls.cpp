#include "adjustment_controls.h"

#include "adjustment_dialog.h"

#include "color_picker_dialog.h"
#include "curve_widget.h"
#include "jump_slider.h"

#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <cmath>
#include <memory>

namespace pictura {

namespace {

QIcon swatch(const QColor& color)
{
    QPixmap pixmap(16, 16);
    pixmap.fill(color);
    QPainter p(&pixmap);
    p.setPen(QColor(0, 0, 0, 160));
    p.drawRect(pixmap.rect().adjusted(0, 0, -1, -1));
    return QIcon(pixmap);
}

// A label and a control on one line.
QWidget* labelled(const QString& label, QWidget* control)
{
    auto* container = new QWidget();
    auto* row = new QHBoxLayout(container);
    row->setContentsMargins(0, 0, 0, 0);
    row->addWidget(new QLabel(label + QLatin1Char(':'), container));
    control->setParent(container);
    row->addWidget(control, 1);
    return container;
}

} // namespace

AdjustmentControls::AdjustmentControls(QWidget* parent)
    : QWidget(parent)
{
    layout_ = new QVBoxLayout(this);
    layout_->setContentsMargins(0, 0, 0, 0);
    layout_->addStretch(1);
}

void AdjustmentControls::setPage(const QStringList& page, bool holdValues)
{
    // Rebuild only for another set of controls; otherwise reload the values,
    // except mid-gesture, when the controls lead.
    QString signature;
    for (const QString& line : page) {
        signature += line.section(QLatin1Char('\t'), 0, 1) + QLatin1Char('\n');
    }
    if (signature != signature_) {
        build(page);
        signature_ = signature;
    } else if (!holdValues) {
        load(page);
    }
}

void AdjustmentControls::insert(QWidget* widget)
{
    layout_->insertWidget(layout_->count() - 1, widget);
}

void AdjustmentControls::clear()
{
    while (layout_->count() > 1) {
        QLayoutItem* item = layout_->takeAt(0);
        delete item->widget();
        delete item;
    }
    rows_.clear();
    groups_ = nullptr;
    curve_ = nullptr;
    curveChannel_ = nullptr;
}

void AdjustmentControls::build(const QStringList& page)
{
    clear();
    loading_ = true;
    for (const QString& line : page) {
        const QStringList cells = line.split(QLatin1Char('\t'));
        const QString& kind = cells.at(0);
        if (kind == QLatin1String("title")) {
            title_ = cells.value(1);
        } else if (kind == QLatin1String("note")) {
            auto* note = new QLabel(cells.value(1));
            note->setWordWrap(true);
            note->setEnabled(false);
            insert(note);
        } else if (kind == QLatin1String("groups")) {
            groups_ = new QComboBox();
            groups_->setObjectName(QStringLiteral("adjustmentGroup"));
            groups_->addItems(cells.value(1).split(QLatin1Char('|')));
            insert(groups_);
            connect(groups_, &QComboBox::currentIndexChanged, this,
                    &AdjustmentControls::showGroup);
        } else if (kind == QLatin1String("curves")) {
            addCurves();
        } else if (kind == QLatin1String("slider")) {
            addSlider(cells);
        } else if (kind == QLatin1String("check")) {
            addCheck(cells);
        } else if (kind == QLatin1String("choice")) {
            addChoice(cells);
        } else if (kind == QLatin1String("color")) {
            addColor(cells);
        }
    }
    showGroup(groups_ ? groups_->currentIndex() : 0);
    loading_ = false;
}

void AdjustmentControls::load(const QStringList& page)
{
    loading_ = true;
    for (const QString& line : page) {
        const QStringList cells = line.split(QLatin1Char('\t'));
        for (Row& row : rows_) {
            if (cells.size() > 4 && row.key == cells.at(1) && row.load) {
                row.load(cells.at(4).toDouble());
            }
        }
    }
    loadCurve();
    loading_ = false;
}

void AdjustmentControls::addSlider(const QStringList& cells)
{
    const QString key = cells.value(1);
    const int decimals = cells.value(8).toInt();
    const double factor = std::pow(10.0, decimals);
    auto* holder = new QWidget();
    auto* column = new QVBoxLayout(holder);
    column->setContentsMargins(0, 0, 0, 4);
    auto* spin = new QDoubleSpinBox();
    spin->setDecimals(decimals);
    spin->setRange(cells.value(6).toDouble(), cells.value(7).toDouble());
    spin->setKeyboardTracking(false);
    // CS6's value row, as every Adjustments dialog lays it out: the label,
    // then a fixed-width field at the right, over the slider.
    spin->setFixedWidth(AdjustmentDialog::kFieldWidth);
    auto* head = new QHBoxLayout;
    head->addWidget(new QLabel(cells.value(2) + QLatin1Char(':'), holder));
    head->addStretch(1);
    head->addWidget(spin);
    column->addLayout(head);
    auto* slider = new JumpSlider(Qt::Horizontal, holder);
    slider->setObjectName(QStringLiteral("adjustmentSlider.") + key);
    slider->setRange(qRound(spin->minimum() * factor), qRound(spin->maximum() * factor));
    column->addWidget(slider);
    insert(holder);

    const auto load = [slider, spin, factor](double v) {
        const QSignalBlocker a(slider);
        const QSignalBlocker b(spin);
        spin->setValue(v);
        slider->setValue(qRound(v * factor));
    };
    load(cells.value(4).toDouble());
    connect(slider, &QSlider::valueChanged, this, [this, key, spin, factor](int v) {
        const QSignalBlocker block(spin);
        spin->setValue(v / factor);
        if (!loading_) {
            emit valueChanged(key, v / factor);
        }
    });
    connect(slider, &QSlider::sliderReleased, this, &AdjustmentControls::gestureEnded);
    connect(spin, &QDoubleSpinBox::valueChanged, this, [this, key, slider, factor](double v) {
        const QSignalBlocker block(slider);
        slider->setValue(qRound(v * factor));
        if (!loading_) {
            emit valueChanged(key, v);
        }
    });
    rows_.push_back({key, cells.value(3).toInt(), holder, slider, load});
}

void AdjustmentControls::addCheck(const QStringList& cells)
{
    const QString key = cells.value(1);
    auto* box = new QCheckBox(cells.value(2));
    box->setObjectName(QStringLiteral("adjustmentCheck.") + key);
    const auto load = [box](double v) {
        const QSignalBlocker block(box);
        box->setChecked(v != 0.0);
    };
    load(cells.value(4).toDouble());
    connect(box, &QCheckBox::toggled, this, [this, key](bool on) {
        if (!loading_) {
            emit valueChanged(key, on ? 1.0 : 0.0);
            emit gestureEnded();
        }
    });
    insert(box);
    rows_.push_back({key, cells.value(3).toInt(), box, box, load});
}

void AdjustmentControls::addChoice(const QStringList& cells)
{
    const QString key = cells.value(1);
    auto* combo = new QComboBox();
    combo->setObjectName(QStringLiteral("adjustmentChoice.") + key);
    combo->addItems(cells.value(6).split(QLatin1Char('|')));
    QWidget* container = labelled(cells.value(2), combo);
    const auto load = [combo](double v) {
        const QSignalBlocker block(combo);
        combo->setCurrentIndex(int(v));
    };
    load(cells.value(4).toDouble());
    connect(combo, &QComboBox::currentIndexChanged, this, [this, key](int i) {
        if (!loading_) {
            emit valueChanged(key, i);
            emit gestureEnded();
        }
    });
    insert(container);
    rows_.push_back({key, cells.value(3).toInt(), container, combo, load});
}

void AdjustmentControls::addColor(const QStringList& cells)
{
    const QString key = cells.value(1);
    auto* button = new QToolButton();
    button->setObjectName(QStringLiteral("adjustmentColor.") + key);
    button->setAutoRaise(true);
    QWidget* container = labelled(cells.value(2), button);
    auto color = std::make_shared<QColor>();
    const auto load = [button, color](double v) {
        *color = QColor::fromRgb(QRgb(uint(v)) | 0xff000000u);
        button->setIcon(swatch(*color));
    };
    load(cells.value(4).toDouble());
    connect(button, &QToolButton::clicked, this, [this, key, color, load]() {
        const QColor picked = ColorPickerDialog::getColor(*color, this, QStringLiteral("Color"));
        if (picked.isValid()) {
            const double rgb = double(picked.rgb() & 0xffffffu);
            load(rgb);
            emit valueChanged(key, rgb);
            emit gestureEnded();
        }
    });
    insert(container);
    rows_.push_back({key, cells.value(3).toInt(), container, button, load});
}

void AdjustmentControls::addCurves()
{
    curveChannel_ = new QComboBox();
    curveChannel_->setObjectName(QStringLiteral("adjustmentCurveChannel"));
    curveChannel_->addItems({QStringLiteral("RGB"), QStringLiteral("Red"), QStringLiteral("Green"),
                             QStringLiteral("Blue")});
    insert(curveChannel_);
    curve_ = new CurveWidget();
    curve_->setObjectName(QStringLiteral("adjustmentCurve"));
    curve_->setMinimumSize(180, 180);
    curve_->setShowHistogram(false);
    insert(curve_);
    connect(curveChannel_, &QComboBox::currentIndexChanged, this, [this]() {
        emit gestureEnded();
        loading_ = true;
        loadCurve();
        loading_ = false;
    });
    connect(curve_, &CurveWidget::curveChanged, this, [this]() {
        if (!loading_) {
            emit curveChanged(curveChannel_->currentIndex(), curve_->pointsText());
        }
    });
    loadCurve();
}

void AdjustmentControls::loadCurve()
{
    if (!curve_ || !curveSource_) {
        return;
    }
    curve_->setPointsText(curveSource_(curveChannel_->currentIndex()));
}

void AdjustmentControls::showGroup(int group)
{
    for (const Row& row : rows_) {
        row.widget->setVisible(row.group < 0 || row.group == group);
    }
}

QWidget* AdjustmentControls::controlForTest(const QString& key) const
{
    for (const Row& row : rows_) {
        if (row.key == key) {
            return row.control;
        }
    }
    if (key == QLatin1String("curve")) {
        return curve_;
    }
    if (key == QLatin1String("group")) {
        return groups_;
    }
    return nullptr;
}

} // namespace pictura

#include "moc_adjustment_controls.cpp"
